# Usability Improvement Backlog

This backlog tracks cleanup after the native workflow module pass. It is scoped
to user-facing API clarity, editor usability, VS Code linting/highlighting, and
documentation consolidation.

## Batch 1: VS Code Highlighting And Wording

Status: implemented in the first cleanup batch.

- Expand TextMate grammar beyond the old keyword/type/numeric split.
- Highlight workflow verbs, schema/type names, units, built-in functions,
  with-block option keys, constants, operators, and string interpolation.
- Add completion entries for public path/config/data types in addition to
  quantity kinds and units.
- Clarify the `englang.diagnosticsBackend` setting wording.
- Add `ide-check` contract coverage for important grammar tokens so keyword
  highlighting does not silently regress.

## Batch 2: Completion Vocabulary Parity

Status: implemented in the third cleanup batch.

- Align LSP snapshot completion with the public workflow vocabulary used by the
  VS Code grammar and fallback completion provider.
- Add LSP and native IDE completion entries for public path/config/data/model/DB
  types, workflow option keys, and common TimeSeries/domain built-ins.
- Extend LSP tests so key workflow tokens such as `render`, `open`,
  `expected_outputs`, `cache_key`, `predict`, and public file/path types stay in
  the completion surface.

## Batch 3: Unsaved Buffer Diagnostics

Status: implemented in the sixth cleanup batch.

- Add `eng-lsp --snapshot-stdin <file.eng>` so editor tooling can lint the
  current buffer text without forcing a save.
- Add debounced VS Code on-change diagnostics for dirty `.eng` buffers through
  the LSP snapshot path.
- Add an `englang.lintOnChange` setting so users can disable live buffer
  diagnostics independently from open/save diagnostics.
- Extend `ide-check` extension contract coverage and LSP integration tests for
  the stdin snapshot path.

## Batch 4: Language Policy Doc Consolidation

Status: implemented in the seventh cleanup batch.

- Replace the stale, duplicated `language_v8.md` body with a short compatibility
  pointer to the current focused policy pages.
- Move language-reference navigation toward `syntax.md`,
  `fast_assignment.md`, and `dimensionless.md` as the detailed sources.
- Remove the historical policy pointer from the parser LLM load map so agents
  load the actual current policy pages first.

## Batch 5: VS Code Migration Quick Fixes

Status: implemented in the eighth cleanup batch.

- Add a VS Code quick fix provider for common syntax migration diagnostics.
- Offer direct edits for `E-SYNTAX-DECL-001` by replacing `:=` with `=`.
- Offer direct edits for `E-STRUCT-ARGS-001` by replacing `struct Args` with
  `args`.
- Extend `ide-check` extension contract coverage so the quick fix provider does
  not silently disappear.

## Batch 6: Workflow Docs De-Duplication

Status: implemented after the native workflow module pass.

- Removed duplicate `*_native.md` workflow pages after the native workflow
  examples became the default documented examples.
- Merged the expected review-surface lists into the primary workflow pages.
- Simplified `docs/workflows/index.md` so each executable workflow appears once.

## Batch 7: Workflow 01 Native Table Transform

Status: implemented after row-field runtime support.

- Replaced the workflow 01 `select_first_row(...)` station lookup with
  `filter` + `require_one`.
- Added runtime evaluation for `require_one` row fields such as
  `station.station_id`.
- Added LSP completion for `require_one` row fields based on the source schema.

## Batch 8: VS Code Setting Wording

Status: implemented after the execution-profile switch.

- Added user-facing enum descriptions for `englang.diagnosticsBackend` so users
  see stable CLI diagnostics vs editor-service snapshot diagnostics instead of
  only internal setting values.
- Added user-facing enum descriptions for `englang.executionProfile` covering
  `normal`, `safe`, and `repro`.
- Updated the VS Code extension README to lead with stable diagnostics and
  editor-service diagnostics before naming the underlying commands.
- Extended `ide-check` contract coverage so setting enum descriptions do not
  disappear silently.

## Batch 9: Workflow Module Status Wording

Status: implemented after module status review.

- Renamed native workflow registry entries from the old preview/seed wording
  to the user-facing `Native workflow support` label.
- Kept the machine-readable status keys stable while updating completion
  details, ReviewDocument status labels, native IDE fallback labels, and the
  generated workflow module docs table.
- Updated status details to refer to executable workflow examples instead of
  implementation fixtures.

## Batch 10: Public Reference Seed Wording

Status: implemented after reference-doc wording audit.

- Replaced implementation-stage uses of `seed` in public reference docs with
  support-scope wording such as native runtime plan, package import metadata,
  table transforms, and native workflow support.
- Left deterministic sampling/reproducibility `seed` wording intact where it is
  part of the user-facing language.

## Batch 11: VS Code Example Runner

Status: implemented after VS Code command review.

- Added `EngLang: Run Example...` to list `examples/official/**/main.eng` and
  `examples/workflows/**/main.eng` from the current workspace.
- Reused the same profile-aware `eng.exe run <file.eng> --profile ... --save-artifacts`
  path as `EngLang: Run Current File`.
- Extended `ide-check` contract coverage so the command and official/workflow
  example discovery stay wired.

## Batch 12: Native IDE Run History And Artifact Links

Status: implemented after native IDE usability review.

- Added a workspace-persistent native IDE run history list with timestamp,
  profile, source file, status, and artifact root.
- Added direct native IDE path opening for workspace-relative files/folders,
  including unsaved in-memory run artifacts by saving the latest artifact set
  before opening the requested path.
- Made artifact, outline, structured read, config promotion, case, model, DB,
  cache, and process-output path rows clickable from the relevant inspectors.
- Extended `ide-check` contract coverage for native IDE UI script/style files
  so the run history and path-link helpers do not disappear silently.

## Batch 13: VS Code Workflow Phrase Highlighting

Status: implemented after VS Code highlighting review.

- Added phrase-aware TextMate scopes for `materialize cases`,
  `apply <step> over`, `collect results`, and `check coverage` so workflow
  nouns and step names are not left as plain identifiers.
- Expanded external-boundary highlighting for HTTP verbs such as `post`,
  `put`, `patch`, `head`, `request`, and `fetch`.
- Added public workflow artifact/status vocabulary such as `OutputManifest`,
  `CacheManifest`, `metadata_ready`, cache hit/miss states, and DB/path option
  keys to the grammar.
- Extended grammar fixtures, expected token checks, and `ide-check` contract
  coverage for the new highlighting surface.

## Batch 14: Native IDE Workflow Node Drawer

Status: implemented after workflow inspector review.

- Added click selection to native IDE workflow DAG cards.
- Added a node detail drawer with rerun decision, prior hash, risk category,
  source-line link, outputs, and incoming/outgoing edges.
- Kept raw node JSON behind a closed advanced toggle so the primary workflow
  view stays summary-first.
- Extended `ide-check` contract coverage for the workflow node drawer UI.

## Batch 15: Public Seed Wording Follow-Up

Status: implemented after public wording audit.

- Replaced public README/user-guide implementation-seed wording with
  implementation-track wording where it describes support boundaries.
- Renamed packaged stdlib module header statuses such as `supported-seed` and
  `supported-native-sqlite-write-seed` to artifact/record-oriented labels.
- Reworded table transform and review module comments to describe supported
  records and external boundary summaries instead of implementation seeds or
  opaque boundaries.
- Left deterministic sampling `seed` vocabulary intact where it is part of the
  user-facing reproducibility API.

## Batch 16: VS Code Script Migration Quick Fix

Status: implemented after VS Code linter review.

- Added a VS Code quick fix for `E-SCRIPT-001` that removes a safe
  `script ... {` wrapper and its matching standalone closing brace, promoting
  the body to the top-level workflow.
- Kept the action conservative: it is offered only when the wrapper line and
  matching closing brace are unambiguous.
- Updated the extension README and `ide-check` contract coverage so the quick
  fix does not disappear silently.

## Batch 17: Native IDE Artifact-Aware Empty States

Status: implemented after native IDE inspector review.

- Added panel-level empty states for Workflow, Quality, Kernel, Effects,
  Network/Cache, DB, Model, and Case panels.
- Each empty state names the missing artifact/payload and the kind of file or
  command that produces it, instead of showing only empty tables.
- Extended native IDE UI contract coverage in `ide-check` for the shared empty
  state helper.

## Batch 18: Native IDE Source Breadcrumbs

Status: implemented after native IDE inspector review.

- Added source-line breadcrumb rows to Network/Cache, DB, Model, and Case
  panels.
- Breadcrumbs collect unique source spans from the panel payload and reuse the
  existing source-line jump behavior.
- Extended native IDE UI contract coverage in `ide-check` for the breadcrumb
  helper and styling.

## Batch 19: Native IDE Raw JSON Toggles

Status: implemented after raw inspector review.

- Added closed advanced raw JSON toggles to Review, Quality, Kernel, Effects,
  Network/Cache, DB, Model, and Case inspectors.
- Replaced inline object JSON strings in summary cells with compact object
  summaries so the first-run view stays table/summary-first.
- Extended native IDE UI contract coverage in `ide-check` for the shared raw
  JSON toggle and compact object summary helpers.

## Batch 20: VS Code Semantic Token Manifest

Status: implemented after semantic highlighting review.

- Declared EngLang-specific VS Code semantic token modifiers for units,
  quantities, axes, time series, uncertainty, workflow boundaries, inputs,
  state, reports, validations, and review risks.
- Added TextMate fallback scope mappings so themes that do not define EngLang
  semantic rules still receive stable color hints.
- Enabled VS Code semantic highlighting by default for `[englang]` files.
- Extended `ide-check` contract coverage for semantic token modifiers, fallback
  scopes, and language default settings.

## Batch 21: LSP Review Risk Semantic Tokens

Status: implemented after compiler-backed semantic token review.

- Connected review risk classification metadata to LSP semantic tokens.
- Added `riskHigh` and `riskMedium` modifiers for diagnostics, schema missing
  policies, explicit process boundaries, file operations, environment
  dependencies, uncertainty declarations, systems, and component assemblies.
- Changed duplicate semantic-token handling to merge modifiers instead of
  dropping later role information for the same token span.
- Added LSP snapshot test coverage for high-risk process tokens and medium-risk
  data-quality, uncertainty, and solver-boundary tokens.

## Batch 22: Native Workflow Zero-Process Wording

Status: implemented after workflow docs review.

- Updated composite workflow docs to describe workflow 1, 2, and 3 as native
  module examples with zero Python or `run command` process execution.
- Replaced stale external-process wording with explicit network, cache, file,
  DB, and artifact boundary records.
- Documented that `dev.bat workflows-test` rejects Python calls, `run command`,
  and nonzero `process_results.json` counts for the three workflow examples.

## Batch 23: Workflow Module Pattern Wording

Status: implemented after workflow module docs review.

- Replaced stale workflow 02 pattern text that described opaque input patching
  and external process runs as the generic core path.
- Described the current executable pattern as native sample, case, template,
  model-card, prediction, DB, and artifact contracts.
- Clarified that external simulators and legacy tools are optional adapters
  above the native contracts, not hidden steps in the workflow examples.

## Batch 24: VS Code Snippet Wording

Status: implemented after snippet wording review.

- Replaced the `system thermal` snippet description's internal
  `preview shape` wording with a direct first-order thermal system model
  description.
- Updated the locally installed VS Code extension snippet file so completions
  use the same wording after reload.

## Batch 25: VS Code Ambiguous Quantity Quick Fix

Status: implemented after VS Code linter review.

- Added quick fixes for `W-QTY-AMBIG-001` diagnostics.
- Each candidate quantity kind gets its own action, so the editor does not pick
  a physical meaning for the user.
- The quick fix rewrites `name = ...` to `name: QuantityKind [unit] = ...`
  only when the warning line has the expected simple binding shape.
- Extended `ide-check` contract coverage for the quantity quick-fix provider.

## Batch 26: Native IDE Problems Filtering

Status: implemented after native IDE Problems panel review.

- Added a Problems text filter that searches severity, code, message, help, and
  line labels alongside the existing severity and diagnostic-code filters.
- Made problem rows clickable source-line jump targets while keeping explicit
  `L<n>` source buttons.
- Extended `ide-check` contract coverage for problem query state, row source
  jump hooks, and Problems panel styles.

## Batch 27: VS Code Workflow Role Semantic Tokens

Status: implemented after VS Code highlighting review.

- Added `model`, `db`, `cache`, and `workflowStep` semantic-token modifiers to
  the LSP and VS Code extension legends.
- Marked model training/evaluation/prediction bindings, cache-backed owner
  bindings and cache options, SQLite bindings and DB writes, and case workflow
  step keywords/options from compiler-backed semantic metadata.
- Added TextMate fallback scope mappings and `ide-check` contract coverage for
  the new role modifiers.
- Added LSP snapshot coverage for model cache records, DB write boundaries, and
  workflow step tokens.

## Batch 28: Native Workflow Support Wording

Status: implemented after API wording review.

- Replaced module status details that framed native workflow support around
  examples with direct wording for implemented runtime paths, listed workflow
  commands/artifacts, and diagnostics for unsupported combinations.
- Updated native IDE fallback wording so the Modules inspector matches compiler
  registry status details.
- Changed workflow module docs to call the current examples executable native
  workflow programs, not fixtures.

## Batch 29: Workflow 02 Native Case Row Rendering

Status: implemented after native workflow completeness review.

- Changed workflow 02 case input rendering to select rows from the generated
  `training_results` sample table through native `filter` and `require_one`
  transforms before rendering templates.
- Renamed the case input template away from placeholder wording and updated its
  text to describe native typed case rendering.
- Removed unused static sample CSV files that were no longer part of the
  workflow execution path.
- Updated workflow module and expected-summary docs to describe selected sample
  rows and workflow programs instead of fixtures.

## Batch 30: Native IDE Module Filtering

Status: implemented after native IDE module inspector review.

- Added Native/Planned/Internal segmented filters and text search to the
  Modules inspector.
- Included module names, status text, purpose, symbols, artifacts, diagnostics,
  examples, and tests in the module search text.
- Counted native workflow registry entries as native in the native IDE
  category view while keeping the machine-readable status keys unchanged.
- Extended `ide-check` contract coverage for module filter state, handlers, and
  styles.

## Batch 31: Public User Docs Fixture Wording

Status: implemented after docs cleanup review.

- Reworded user-facing uncertainty and composite workflow docs to describe
  scoped workflow examples and deterministic offline inputs instead of generic
  fixtures.
- Reworded the user guide so advanced/internal examples are clearly
  inspection-only or repository regression examples, not supported tutorials.
- Kept network fixture terminology where it describes an actual offline API
  boundary contract.

## Batch 32: Built-In Helper Semantic Highlighting

Status: implemented after VS Code highlighting review.

- Made `eng-lsp` emit semantic `function` tokens for native workflow helper
  names that TextMate already recognized, including `require_one`,
  `regression_table`, `evaluate`, `model_card`, and `predict`.
- Added role modifiers for built-in helper families, including model,
  uncertainty, validation, and workflow-step helpers.
- Added semantic namespace tokens and TextMate fallback scope coverage for
  module/package names such as `eng.std.domains.thermal`.
- Extended LSP unit tests, VS Code grammar fixtures, generated TextMate grammar,
  README wording, and `ide-check` contract coverage.

## Batch 33: Saved Artifact Command Examples

Status: implemented after public command example review.

- Updated the root README current-command sequence so the CSV plot run matches
  the immediately following `build\result\result.engres` view command.
- Updated the official CSV plot example run commands to include
  `--save-artifacts` because the example description highlights saved
  PlotSpec, SVG, report, review, and result artifacts.
- Left developer/reference examples unchanged where they intentionally contrast
  in-memory runs with saved artifact runs.

## Batch 34: Missing Unit Quick Fix

Status: implemented after VS Code linter quick-fix review.

- Added a VS Code quick fix for `E-DIM-ADD-*` diagnostics that can safely insert
  a missing unit suffix onto a bare numeric literal.
- The fix uses explicit compiler help such as `kW` or `K` when present, then
  falls back to a unit already visible on the diagnostic line.
- Kept the edit narrow: it only inserts a suffix after numeric literals on the
  diagnostic line, and it does not infer a unit when no stable hint exists.
- Updated extension README wording and `ide-check` contract coverage.

## Batch 35: Schema Annotation Quick Fix

Status: implemented after public annotation diagnostic review.

- Made `E-PUBLIC-ANNOTATION-001` compiler help use the actual schema column
  name, unit, and inferred quantity kind when the unit registry can resolve it.
- Added a VS Code quick fix that converts a schema-local `name = value unit`
  line into `name: QuantityKind [unit]`.
- Kept the edit guarded by the compiler help and the source-line left-hand name
  so the extension does not duplicate quantity inference logic.
- Extended compiler tests, extension README wording, and `ide-check` contract
  coverage.

## Batch 36: Workflow Helper Completion Details

Status: implemented after LSP completion vocabulary review.

- Expanded workflow helper completion details so table, sampling, case, model,
  uncertainty, and timeseries helpers expose concrete module-oriented wording.
- Changed completion ordering so workflow helpers such as `require_one`,
  `regression_table`, and `predict` keep `function` completion kind instead of
  being deduplicated as generic keywords.
- Added LSP snapshot assertions for helper kind/detail parity across table,
  sampling, model, and timeseries helpers.

## Batch 37: Workflow 01 Network Value Resolution

Status: implemented after native workflow source review.

- Changed workflow 01 to call `http get args.api_url` instead of hardcoding the
  endpoint beside an unused URL arg.
- Routed the weather API cache key through stable `args.region` and `args.year`
  parts instead of fixed demo literals.
- Added runtime network-boundary query resolution so non-redacted query values
  can use native runtime selections such as `selected_station_id -> STN001`.
- Extended compiler and runtime workflow tests to pin args-driven URL/cache
  behavior and resolved station query artifacts.

## Batch 38: VS Code Doc Comment Continuation

Status: implemented after editor language-configuration review.

- Added a VS Code `onEnterRules` language configuration entry so `///`
  documentation comments continue automatically on Enter.
- Kept `#` as the normal line-comment toggle while treating `///` as a doc
  comment editing affordance instead of the default toggle style.
- Extended `ide-check` extension contract coverage so doc-comment continuation
  cannot silently disappear from the packaged extension.

## Batch 39: File Mutation Quick Fixes

Status: implemented after VS Code linter quick-fix review.

- Added VS Code quick fixes for `E-FS-CONFIRM-001` and `E-FS-DELETE-001`.
- The fixes add `with { confirm = true }` for unconfirmed `move`/`delete`
  statements and add `recursive = true` plus `confirm = true` for directory
  deletes when needed.
- The edit reuses an attached multiline `with` block when one exists, avoiding
  duplicate options and keeping filesystem mutation policy visible in source.
- Extended extension README wording and `ide-check` quick-fix contract coverage.

## Batch 40: With-Block Option Completion

Status: implemented after LSP completion context review.

- Added position-aware LSP completion for `with { ... }` blocks.
- Completion now uses the owner statement to prefer relevant options for
  network requests, downloads, process boundaries, file mutations, template
  renders, case materialization, coverage checks, sampling, and model helpers.
- Existing options in the same multiline `with` block are skipped so users do
  not get duplicate `confirm`, `cache_key`, or similar suggestions.
- Expanded option completion vocabulary for supported public keys such as
  `confirm`, `recursive`, `cache_dir`, `expected_step`, `response_body_limit`,
  `start`, `end`, and `year`.

## Batch 41: LSP Diagnostic Source Ranges

Status: implemented after VS Code underline review.

- LSP diagnostics now carry source-aware `start_character` and `end_character`
  values instead of always underlining the first character of the line.
- Dimensionless arithmetic diagnostics highlight the offending `+`/`-`
  operator, schema fast-assignment diagnostics highlight `=`, and filesystem
  mutation diagnostics target `move`/`delete`.
- Generic diagnostics fall back through backticked message/help text, the first
  identifier, and then the first visible token so VS Code ranges stay useful
  even before compiler diagnostics have first-class spans.
- Added JSON-level regression coverage for arithmetic and schema diagnostic
  ranges.

## Batch 42: TextMate Workflow Phrase Coverage

Status: implemented after syntax-highlighting consistency review.

- Added phrase-aware TextMate scopes for `http get`/`http post` style request
  boundaries so network request verbs remain grouped before semantic tokens
  arrive.
- Added a report option phrase scope for `unit y = ...`, which previously did
  not get the same option-key treatment as normal `with { unit = ... }` keys.
- Pinned `return`, `to`, HTTP request phrases, and report unit-axis syntax in
  grammar smoke expectations.

## Batch 43: Schema Modifier Highlighting

Status: implemented after schema fixture highlighting review.

- Added a TextMate scope for the schema `index` modifier so time/index columns
  do not appear as plain identifiers before semantic highlighting arrives.
- Added bracketed DateTime format marker highlighting for `[iso8601]`,
  `[unix]`, `[epoch]`, and `[utc]`.
- Pinned schema modifier and format marker coverage in grammar smoke fixtures.

## Batch 44: Semantic Legend Contract

Status: implemented after VS Code/LSP metadata drift review.

- Extended `ide-check` so the VS Code extension's `SEMANTIC_TOKEN_TYPES` and
  `SEMANTIC_TOKEN_MODIFIERS` must match the `eng_lsp` legend exactly.
- Added a manifest check that every nonstandard LSP semantic-token modifier is
  declared in `package.json`, keeping VS Code theme fallback metadata aligned
  with compiler-backed semantic tokens.
- This keeps future highlighting additions from landing in LSP only or VS Code
  only without failing the editor contract gate.

## Batch 45: LSP Snapshot Reference Refresh

Status: implemented after editor-token documentation review.

- Updated the LSP snapshot reference so its top-level shape includes semantic
  tokens, document symbols, and folding ranges.
- Replaced stale coarse diagnostic range wording with the current source-aware
  UTF-16 range behavior.
- Added a semantic token section documenting the legend, token coordinates,
  EngLang-specific modifiers, and the `ide-check` parity gate with the VS Code
  extension.

## Batch 46: VS Code Current-File Definition Provider

Status: implemented after editor navigation review.

- Added a VS Code definition provider backed by the unsaved-buffer
  `eng-lsp --snapshot-stdin` document symbols.
- Definition lookup now resolves current-file top-level and nested document
  symbols such as schema fields, class fields, component ports, and object
  members when the snapshot exposes them.
- Updated extension docs and `ide-check` contract coverage so the provider does
  not silently disappear.

## Batch 47: LSP Definition Source Ranges

Status: implemented after LSP navigation range review.

- `textDocument/definition` now returns the actual UTF-16 source range of the
  definition label instead of the previous `0..1` placeholder range.
- Shared hover/definition symbol matching now prefers exact names and then
  falls back to the final dotted segment, reducing inconsistent behavior for
  dotted references.
- The stdio LSP regression test now pins both variable and function definition
  ranges so editor navigation cannot silently lose source precision.

## Batch 48: Cross-File Import Definitions

Status: implemented after imported function navigation review.

- `textDocument/definition` now falls back from the current document to static
  file imports such as `use "thermal.eng"` when the selected symbol is defined
  in an imported file.
- Definition lookup uses parsed AST definition nodes before computing source
  ranges, avoiding accidental jumps to same-line usages that only contain the
  same label.
- The stdio LSP regression test now pins `main.eng` `heat_loss(...)` navigation
  to the actual `thermal.eng` `fn heat_loss` source range.

## Batch 49: TextMate Keyword Consistency

Status: implemented after compiler keyword/TextMate coverage review.

- Added TextMate coverage for `predict model using`, `output` declarations,
  `on` blocks, `import`, and deprecated `script`/`struct` keywords.
- Extended grammar fixtures and smoke expectations from 94 to 101 token checks
  so these keyword groups cannot silently drop back to plain identifier colors.
- Regenerated the VS Code TextMate grammar and synced the grammar files into
  the installed local extension folder.

## Batch 50: Workflow API Payload Contract

Status: implemented after workflow 01 native-contract review.

- Added `WeatherApiRecord` and `WeatherApiPayload` schemas to workflow 01 so
  the API fixture JSON is validated through native `read json` +
  `promote json ... as WeatherApiPayload`.
- Verified the saved run records `api_contract` as a validated config
  promotion and still has `process_results.json.process_count = 0`.
- Follow-up completed in Batch 51: direct JSON-record-to-table materialization
  now replaces the old CSV weather-table fixture in workflow 01.

## Batch 51: Workflow JSON Records Table Promotion

Status: implemented after workflow 01 native-table review.

- Added native `promote json records <payload>.<array> as <Schema>` analysis
  for `read json` payload bindings, including row count, source hash, headers,
  `source_format = json_records`, and review metadata.
- Runtime materialization now turns JSON record arrays into typed
  `RuntimeTable` values in schema column order, then reuses existing table
  diagnostics, DateTime coverage, missing policies, constraints, and
  provenance paths.
- Updated workflow 01 to remove the `weather_hourly` CSV arg and promote
  `api_payload.records` as `WeatherApiRecord`; the workflow contract test
  now asserts `source:json_records:weather` and absence of
  `sample_weather_hourly.csv` in the result.

## Batch 52: JSON Records Promotion Highlighting

Status: implemented after syntax-highlighting TODO review.

- Added a TextMate workflow phrase for `promote json records` so the new native
  JSON-record table syntax is highlighted as a coherent workflow construct
  instead of leaving `records` as an uncolored identifier.
- Added `records` to the workflow keyword group and pinned
  `payload.records` property-path coverage in the grammar fixture.
- Regenerated the VS Code grammar, synced the local installed extension
  grammar, and verified `vscode-grammar-test`, `lsp-check`, and `ide-check`.

## Batch 53: JSON Records LSP Metadata Alignment

Status: implemented after grammar/LSP consistency review.

- Added `records` and `promote json records` to the LSP completion surface so
  the new native table promotion is discoverable from VS Code and snapshot
  completions.
- Marked `promote json records` semantic tokens with the `workflowStep`
  modifier and changed document-symbol detail from CSV wording to
  `json_records as <Schema>` for JSON-record table promotions.
- Added an LSP snapshot regression that pins completion, semantic token, and
  document-symbol behavior for the JSON records promotion syntax.

## Batch 54: LSP Editor Metadata Export

Status: implemented after VS Code/LSP drift review.

- Added `eng-lsp --editor-metadata`, a machine-readable editor contract export
  for the LSP semantic-token legend and completion seed.
- The export now includes string completion kinds, LSP numeric kinds, details,
  and a seed count so extension checks can catch missing workflow phrases such
  as `promote json records` before users see inconsistent highlighting or
  completion behavior.
- Extended LSP stdio coverage, `lsp-check`, and VS Code extension docs to
  verify the metadata export and keep it connected to editor maintenance.

## Batch 55: TextMate/LSP Option Coverage Guard

Status: implemented after keyword-highlighting drift review.

- Added missing TextMate highlighting for LSP workflow option completions
  `cache_dir` and `response_body_limit`.
- Extended the VS Code grammar fixture and expected token smoke from 104 to
  106 checks so both options are highlighted as with-block properties.
- Strengthened `vscode-grammar-test` to compare the TextMate grammar source
  against LSP keyword, workflow builtin, workflow option, and public type
  completion seed constants, catching future editor-color drift earlier.

## Batch 56: Native HTTP Response JSON Promotion

Status: implemented after workflow 1 native-dataflow review.

- Workflow 01 now reads its typed API payload from `api_response.body` instead
  of reading `args.api_fixture` directly after the network boundary.
- Compiler schema analysis now treats fixture-backed `http get` response
  bodies as structured JSON sources for `read json <response>.body`, enabling
  config promotion and JSON-record table promotion to follow the native
  network/cache boundary.
- Added compiler and workflow contract coverage so `source_value` stays
  `api_response.body`, review metadata records `read json api_response.body`,
  and workflows 1-3 still run with `process_count = 0`.

## Batch 57: Generated VS Code Editor Metadata

Status: implemented after extension source-of-truth review.

- Added a VS Code editor metadata generator that writes package-local
  `generated/editor` JSON from `eng-lsp --editor-metadata`.
- The VS Code extension now loads its semantic-token legend from generated
  metadata instead of hardcoded JavaScript arrays.
- `ide-check` verifies the generated metadata is in sync with LSP constants
  and that the extension does not reintroduce hardcoded semantic legend arrays.

## Batch 58: Generated VS Code Completion Fallback

Status: implemented after extension completion source-of-truth review.

- The VS Code completion provider now uses live `eng-lsp --completion-stdin`
  results first and falls back to the generated editor metadata completion seed
  when no live/review-cache completion payload is available.
- The fallback uses generated LSP completion kind IDs instead of a JavaScript
  keyword/type/unit table, keeping extension vocabulary tied to LSP metadata.
- `ide-check` now verifies the generated completion seed mirror, required
  workflow/data completion labels, `lsp_kind` presence, and extension fallback
  wiring.

## Batch 59: TextMate Unit And Quantity Drift Guard

Status: implemented after unit highlighting consistency review.

- Added TextMate fallback coverage for compiler-known ASCII unit aliases
  `W/m^2` and `people/m2`, so unit highlighting stays consistent when semantic
  tokens are unavailable or disabled.
- Extended the grammar smoke guard to compare TextMate fallback labels against
  compiler `UNIT_INFOS` and `QUANTITY_COMPLETIONS`, catching unit and quantity
  drift before release.
- Added grammar fixture expectations for full quantity literals and standalone
  unit tokens using those aliases.

## Batch 60: VS Code User-Facing Wording Cleanup

Status: implemented after settings and command wording review.

- Reworded VS Code setting descriptions around diagnostics, live linting,
  semantic highlighting, and execution profiles so users see file/live-check
  behavior instead of editor-service implementation details.
- Renamed last-run artifact command titles to include "Last Run", reducing
  ambiguity between current-file review output and saved run artifacts.
- Renamed the semantic-token debug command title to "Inspect Highlight Tokens"
  and extended `ide-check` to guard these wording choices.

## Batch 61: Workflow Module Seed Wording Cleanup

Status: implemented after native workflow contract recheck.

- Verified `dev.bat workflows-test` still proves workflows 01, 02, and 03 run
  without Python or external process execution and keep `process_count = 0`.
- Renamed compiler tests and module-registry entries that described implemented
  DB writes and expectation-suite records as seeds.
- Reworded stdlib boundary notes and workflow 03 documentation so user-facing
  docs describe records, contracts, and narrow evidence instead of
  implementation seeds or generic fixtures.

## Batch 62: VS Code Review Risk Decorations

Status: implemented after review/risk visibility review.

- Added high/medium review-risk editor decorations that render as a subtle
  left border plus overview-ruler mark using existing compiler review risk
  records and semantic-token risk modifiers.
- Added `englang.reviewRiskDecorations.enabled` so users can hide those markers
  without disabling diagnostics or semantic highlighting.
- Extended `ide-check` so VS Code risk-decoration wiring and user-facing
  setting text stay covered.

## Batch 63: Native IDE Semantic Highlight Overlay

Status: implemented after native IDE highlight parity review.

- Connected native IDE check results to the same `eng_lsp` semantic-token
  snapshot used by VS Code editor metadata.
- Added a token-colored editor overlay and a Highlight sidebar tab that exposes
  the current semantic-token legend, type/modifier counts, and token ranges.
- Extended `ide-check` and Rust regression coverage so native IDE semantic
  token payloads, overlay wiring, and risk/unit highlight styles stay guarded.

## Batch 64: Native IDE Completion Seed Source

Status: implemented after native IDE completion source review.

- Replaced native IDE hardcoded keyword/type/workflow completion constants with
  the shared `eng_lsp` editor completion seed used by VS Code metadata.
- Kept native IDE multi-line snippets for larger editing patterns while
  deduplicating them by label against the LSP seed.
- Extended `ide-check` and Rust regression coverage so native IDE completion
  labels mirror the LSP seed and do not reintroduce duplicate completion
  constants.

## Batch 65: Native IDE Caret Token Insight

Status: implemented after native IDE highlight usability review.

- Added LSP hover payloads to native IDE check results alongside semantic
  tokens.
- Added editor-meta caret insight that shows line/column, semantic token
  type/modifiers, and quantity/unit hover detail for the current caret token.
- Extended `ide-check` and Rust regression coverage so native IDE hover payload
  wiring and caret token insight helpers remain covered.

## Batch 66: VS Code Live Hover Snapshot

Status: implemented after VS Code hover parity review.

- Changed the VS Code hover provider to read the current unsaved buffer through
  `eng-lsp --snapshot-stdin`, matching diagnostics, semantic tokens,
  completion, symbols, folding, and current-file definition behavior.
- Matched dotted and namespaced hover names such as `where.Q_for_energy`,
  `Thermal.T`, and component ports when the caret is on the local token.
- Extended `ide-check` and README coverage so live hover snapshot wiring,
  kind/status display, and namespace-aware matching stay documented.

## Batch 67: Richer Keyword Semantic Highlighting

Status: implemented after VS Code keyword-color consistency review.

- Added a compiler-owned `solver` semantic-token modifier and mapped it through
  generated editor metadata, VS Code fallback scopes, and the native IDE
  highlight overlay.
- Expanded LSP keyword modifiers so report, validation, side-effect, external,
  workflow-step, deprecated, and solver keywords that TextMate already colors
  also carry semantic modifiers.
- Added LSP regression coverage and `ide-check` contract guards so semantic
  keyword colors stay aligned with TextMate fallback scopes.

## Batch 68: VS Code Option Value Quick Fixes

Status: implemented after VS Code linter quick-fix review.

- Added VS Code quick fixes for invalid network retry, timeout, and response
  body-size options, replacing only the invalid option value on the diagnostic
  line.
- Added VS Code quick fixes for process retry, timeout, and `allow_failure`
  option diagnostics using compiler-supported example values.
- Extended README and `ide-check` contract coverage so option-value quick fixes
  remain part of the editor surface.

## Batch 69: VS Code Cross-File Definition Bridge

Status: implemented after VS Code navigation review.

- Added `eng-lsp --definition-stdin <file.eng> <line> <character>` so editor
  integrations can resolve definitions from the current unsaved buffer while
  reusing the existing LSP static file import resolver.
- Updated the VS Code definition provider to ask the LSP bridge first, allowing
  static file imports to jump to imported source files, with current-buffer
  document-symbol fallback retained when live lookup is unavailable.
- Normalized Windows verbatim paths in LSP file URIs so imported definitions
  produce VS Code-friendly `file:///C:/...` locations.
- Added regression and `ide-check` coverage so the CLI bridge, VS Code provider,
  and documented definition behavior stay aligned.

## Batch 70: LSP Semantic Token Range Requests

Status: implemented after persistent LSP capability review.

- Changed `eng-lsp` initialize capabilities to advertise
  `semanticTokensProvider.range = true` alongside full semantic tokens.
- Added a `textDocument/semanticTokens/range` handler that reuses compiler-owned
  snapshot semantic tokens and filters them to the requested source range before
  returning standard LSP delta-encoded token data.
- Extended stdio and `ide-check` coverage so range semantic tokens, full tokens,
  and the advertised LSP capability stay in sync.

## Batch 71: LSP Syntax Migration Code Actions

Status: implemented after linter quick-fix protocol review.

- Added `codeActionProvider` capability to `eng-lsp` with quick-fix support for
  syntax migration diagnostics.
- Implemented `textDocument/codeAction` edits for `:=` to `=`, `struct Args`
  to `args`, and equation-block `==` to `eq`, returning standard WorkspaceEdit
  payloads from the current document diagnostics context.
- Extended stdio and `ide-check` coverage so persistent LSP clients retain the
  migration quick fixes that were previously available only in the VS Code
  extension layer.

## Batch 72: Compiler-Owned Editor Formatting

Status: implemented after syntax-aware editor feature review.

- Added `eng-lsp --format-stdin <file.eng>` so editor integrations can format
  the current unsaved buffer through the compiler-owned formatter.
- Added persistent LSP `documentFormattingProvider` and
  `textDocument/formatting`, returning a full-document `TextEdit` only when the
  compiler formatter changes the source.
- Added a VS Code document formatting provider that calls the same LSP stdin
  bridge, avoiding a separate JavaScript indentation or block-formatting
  implementation.
- Extended stdio tests, README, and `ide-check` contract coverage so formatting
  stays aligned across CLI, LSP, and VS Code.

## Batch 73: LSP Script Wrapper Quick Fix

Status: implemented after linter quick-fix parity review.

- Added a persistent LSP `textDocument/codeAction` quick fix for
  `E-SCRIPT-001`, matching the existing VS Code quick fix that promotes a safe
  `script main { ... }` body to top-level workflow code.
- Kept the action conservative by requiring a script wrapper start line, a
  matching block end, and a standalone closing `}` before returning the
  multi-edit WorkspaceEdit.
- Extended stdio and `ide-check` coverage so persistent LSP clients retain this
  migration path instead of depending on VS Code-only JavaScript logic.

## Batch 74: Workflow 01 Native Row Selection Cleanup

Status: implemented after native workflow API review.

- Removed the legacy `select_first_row(...)` station lookup from workflow 01.
- Routed the API query through the `filter` + `require_one` row object via
  `station.station_id`, keeping `selected_station_id` only as a report-friendly
  alias.
- Added a `workflows-test` guard so native workflow examples cannot reintroduce
  the legacy table helper while claiming native row selection.

## Batch 75: Workflow 02 Native Summary Values

Status: implemented after native surrogate workflow review.

- Removed fixed summary literals from workflow 02.
- Routed `annual_electricity`, `peak_cooling`, and `unmet_hours` through the
  selected `case_001` workflow row so the CSV summary matches the native table
  path and rendered case input.
- Extended workflow smoke coverage so `workflow_summary.csv` cannot regress to
  the old fixed `12800.0,14.2,0.0` row while claiming native sampling.

## Batch 76: Workflow 03 Native Output Artifacts

Status: implemented after native uncertainty workflow review.

- Added native mean, peak, integrated energy, and time-axis coverage bindings to
  workflow 03.
- Added `outputs/sensor_summary.csv` and `outputs/sensor_quality_summary.txt`
  so the sensor workflow produces reviewable user output artifacts, not only
  saved-run metadata.
- Extended workflow smoke coverage to reject Python/external process helpers in
  workflows 1-3 and to require workflow 03 CSV/text output manifest entries.

## Batch 77: VS Code Plot And Workflow Phrase Highlighting

Status: implemented after VS Code grammar/LSP consistency review.

- Added `confidence_band`, `unit`, and `title` to LSP workflow option
  completions for plot `with` blocks.
- Added TextMate phrase scopes for `export summary to csv` and `write text` so
  common workflow phrases do not leave middle keywords uncolored.
- Extended grammar fixtures and smoke expectations to cover
  `confidence_band`, `write text`, and `export summary to csv`.

## Batch 78: LSP Workflow Phrase Semantic Alignment

Status: implemented after TextMate/LSP semantic overlay review.

- Added `summary` to compiler-backed LSP keyword completions so
  `export summary to csv` is not TextMate-only.
- Marked `text`, `csv`, `json`, and `toml` workflow phrase tokens with the
  `workflowStep` semantic modifier to match TextMate fallback highlighting.
- Added `export summary to csv` to phrase completions and extended semantic
  token tests for `summary`, `text`, `csv`, `json`, and `toml`.

## Batch 79: VS Code Shared Snapshot Reuse

Status: implemented after VS Code live-editor performance review.

- Added per-document-version snapshot promise reuse in the VS Code extension so
  semantic tokens, symbols, folding, hover, definition fallback, and token
  debug do not spawn duplicate `eng-lsp --snapshot-stdin` processes for the
  same buffer.
- Clear cached snapshot promises on document changes and close.
- Keep shared snapshot subprocesses independent from individual provider
  cancellation so one canceled request does not poison other editor features.

## Batch 80: LSP Linter Quick Fix Parity

Status: implemented after VS Code-only quick-fix parity review.

- Added persistent LSP `textDocument/codeAction` quick fixes for ambiguous
  quantity annotations, missing unit hints, schema-column annotation migration,
  filesystem confirmation/recursive-delete options, and invalid net/process
  option values.
- Kept the edits line-based and conservative, matching the existing VS Code
  quick-fix behavior without adding JavaScript-only linter semantics.
- Extended stdio and `ide-check` coverage so non-VS Code LSP clients receive
  the same common linter repair actions.

## Batch 81: VS Code LSP Code Action Bridge

Status: implemented after VS Code/LSP quick-fix source-of-truth review.

- Added `eng-lsp --code-actions-stdin <file.eng>` so editor clients can request
  quick fixes for the current unsaved buffer without running a persistent
  server session.
- Updated the VS Code code action provider to prefer LSP-owned quick fixes and
  convert LSP `WorkspaceEdit` payloads into VS Code edits.
- Kept the existing JavaScript quick fixes as a local fallback when the LSP
  bridge is unavailable or returns no matching actions.

## Batch 82: Native IDE Token Range Navigation

Status: implemented after native IDE highlight-panel workflow review.

- Added exact semantic-token range navigation from the native IDE Highlight
  panel so token rows select the token span, not only the containing line.
- Reworked line selection through a shared source-line range helper that
  preserves CRLF/LF offsets before updating textarea selection.
- Extended `ide-check` contract coverage so token-range buttons and selection
  helpers remain wired.

## Batch 83: VS Code LSP Quick Fix Range Matching

Status: implemented after VS Code/LSP quick-fix bridge review.

- Tightened VS Code quick-fix filtering so LSP diagnostics must match the full
  line and character range of the active VS Code diagnostic before their code
  actions are shown.
- Extended `ide-check` contract coverage so the LSP action bridge does not
  regress to coarse same-line matching.

## Batch 84: Native Workflow Python/Process Guards

Status: implemented after workflow 01/02/03 native-source audit.

- Confirmed workflow examples 01, 02, and 03 do not call Python or `run command`
  in their `.eng` sources and still have smoke coverage for `process_count = 0`.
- Tightened Rust example smoke and PowerShell `workflows-test` source guards so
  Python script paths, notebook/data-science library markers, subprocess
  markers, process adapters, and legacy seeded row selection helpers are blocked
  before native workflow examples can regress.

## Batch 85: User Docs Navigation And Publishing Wrapper

Status: implemented after docs cleanup review.

- Added `.\dev.bat user-docs-markdown` as the user-guide assembly entrypoint so
  user documentation no longer leads with a direct Python command.
- Clarified that repo-local Python/OODocs are optional publishing tooling, not
  runtime, workflow, test, or package-smoke dependencies.
- Marked `docs/workflows/index.md` as a workflow subguide under
  `docs/README.md`, not a parallel global documentation index.

## Batch 86: Native IDE Variable Source Navigation

Status: implemented after native IDE inspector workflow review.

- Added source-line navigation buttons to runtime Variables and Args source
  cells so repeated run/debug inspection can jump back to the defining source
  line without switching to another inspector.
- Kept variable row expansion separate from source navigation, so clicking a
  source line does not toggle the variable detail row.
- Extended `ide-check` contract coverage for the shared variable source cell
  and styling.

## Batch 87: Native IDE Source-Line Field Normalization

Status: implemented after native IDE source-jump audit.

- Normalized native IDE source-line extraction through a shared helper so
  inspector links recognize `source_span.line`, `sourceSpan.line`,
  `source_line`, `sourceLine`, and plain `line` records.
- Reused that helper for source breadcrumbs, keeping Network/Cache, DB, Model,
  Case, and other review inspectors from losing links when artifact payloads
  use snake-case or camel-case source-line fields.
- Extended `ide-check` contract coverage for the normalized source-line field
  variants.

## Batch 88: VS Code Review Source-Line Field Normalization

Status: implemented after VS Code review-panel source-jump audit.

- Normalized VS Code review-panel source-line extraction so clickable source
  links recognize `source_span.line`, `sourceSpan.line`, `source_line`,
  `sourceLine`, and plain `line` records.
- Reused the same normalized helper for review risk decorations so gutter and
  overview markers do not disappear when risk records use span-shaped source
  metadata.
- Extended `ide-check` contract coverage for the VS Code normalized source-line
  tokens.

## Batch 89: Native Workflow Phrase Highlighting

Status: implemented after TextMate workflow fixture review.

- Added phrase-aware TextMate scopes for `read json`/`read toml`/`read text`,
  `open sqlite`, and `write <table> to db.table(...)` so native
  workflow I/O and DB operations remain highlighted as coherent actions.
- Extended grammar smoke expectations for workflow 01/02 constructs that
  previously depended on generic keyword/operator scopes.
- Regenerated the VS Code TextMate grammar from the readable source grammar.

## Batch 90: Native Sampling/Predict Phrase Highlighting

Status: implemented after workflow 02 model/sampling fixture review.

- Added phrase-aware TextMate scope coverage for `sample lhs`, `sample grid`,
  and `sample random`.
- Generalized `predict <model> using` highlighting so model variables such as
  `surrogate_model` are scoped as workflow-local operands instead of requiring
  the literal word `model`.
- Extended grammar smoke fixtures/expectations and regenerated the VS Code
  grammar.

## Batch 91: Sampling Semantic Token Parity

Status: implemented after LSP semantic-token review.

- Marked `sample`, `lhs`, `grid`, and `random` semantic function tokens with
  the `workflowStep` modifier so semantic highlighting matches the TextMate
  sampling phrase scope.
- Marked `normal` and `uniform` distribution helpers with the `uncertain`
  modifier while keeping their default-library function role.
- Extended LSP snapshot coverage so native IDE and VS Code semantic overlays
  keep sampling/distribution colors in sync.

## Batch 92: Sampling Seed Quick Fix

Status: implemented after VS Code/LSP linter quick-fix review.

- Added an LSP-owned quick fix for `E-SAMPLING-SEED-INVALID` diagnostics that
  rewrites invalid `seed` option values to `seed = 42`.
- Mirrored the same quick fix in the VS Code local fallback for cases where the
  LSP code-action bridge is unavailable.
- Extended stdio and stdin code-action regression coverage and updated the
  extension README quick-fix surface.

## Batch 93: Workflow Gate Wording Cleanup

Status: implemented after docs cleanup review.

- Reworded workflow test-gate docs so native SQLite prediction writes are
  described as the current DB write path instead of a DB seed.
- Narrowed the remaining-gap wording to live network execution, cache policy,
  broader sampling distributions/design policies, case-runner scheduling, DB
  reads/queries, and public model training syntax.

## Batch 94: Sampling Seed Diagnostic Wording

Status: implemented after diagnostic API wording review.

- Split invalid sampling seed values into `E-SAMPLING-SEED-INVALID` while
  keeping `E-SAMPLING-SEED-MISSING` for repro-profile samples that omit a seed.
- Kept invalid seed values on the option-value replacement quick-fix path while
  reserving missing seed diagnostics for a broader insertion quick fix.
- Updated workflow module and CLI diagnostic references so the two conditions
  are no longer conflated.

## Batch 95: Sampling Alias Highlighting

Status: implemented after sampling grammar/semantic alias review.

- Extended TextMate phrase coverage for compiler-supported `sample uniform`,
  `sample latin_hypercube`, and `sample latin-hypercube` aliases.
- Added `latin_hypercube` to the shared LSP editor vocabulary and generated
  grammar builtin coverage.
- Made `uniform` keep its uncertainty function role while also receiving the
  `workflowStep` semantic modifier specifically in `sample uniform` context.
- Pinned both underscore and hyphen Latin-hypercube aliases in TextMate grammar
  smoke coverage.

## Batch 96: Missing Sampling Seed Quick Fix

Status: implemented after repro-profile quick-fix review.

- Added LSP and VS Code fallback quick fixes that insert `seed = 42` for
  `E-SAMPLING-SEED-MISSING` diagnostics on sample-generation owner lines.
- Kept `E-SAMPLING-SEED-INVALID` on the option-value replacement path so
  invalid option lines still rewrite only the bad value.
- Extended code-action regression coverage with a synthetic repro-profile
  diagnostic and updated the VS Code README quick-fix surface.

## Batch 97: Editor Token Scope Guide

Status: implemented after syntax-highlighting TODO review.

- Added `docs/internal/editor/token_scopes.md` as the maintainer-facing contract
  for TextMate scope naming, semantic-token modifiers, VS Code fallback
  mappings, and update commands.
- Linked the VS Code extension README to the new guide so highlighting and
  semantic-token drift has a discoverable maintenance path.
- Confirmed the current workflow 01/02/03 examples have no Python, `.py`, or
  `run command` usage and that `eng test examples` enforces this through the
  native workflow source scan.

## Batch 98: Write Text Interpolation Diagnostics

Status: implemented after string-interpolation TODO review.

- Added compiler/LSP diagnostics for invalid `write text` string interpolation
  placeholders, matching the existing `print`/`log` semantic checks.
- Split the user-facing codes as `E-WRITE-FMT-*` so text-output interpolation
  errors are not reported as print-format errors.
- Added compiler regression coverage for unresolved placeholders and
  incompatible requested units in `write text` templates.

## Batch 99: Declaration Name Grammar Pins

Status: implemented after declaration/function-name highlighting review.

- Extended the VS Code grammar smoke harness so capture scopes are tested
  directly, not only their parent phrase scopes.
- Pinned declaration-name captures for `schema`, `system`, `domain`,
  `component`, `class`, `fn`, `method`, `const`, and `test` names.
- Added fixture coverage for TODO examples such as `class Zone` and
  `component Coil`.
- Updated the editor token-scope guide with the declaration-name capture
  contract.

## Batch 100: Block Keyword Boundary Highlighting

Status: implemented after args/report block highlighting review.

- Narrowed the TextMate block keyword scope so `args`, `report`, `with`, `on`,
  and related block words are highlighted as block openers only when followed by
  `{`.
- Added grammar smoke coverage for `args {`, `report {`, and `on {` so block
  keywords do not collide with option keys such as `args =`.
- Updated the editor token-scope guide to document block opener scope intent.

## Batch 101: Punctuation Scope Split

Status: implemented after bracket/punctuation highlighting review.

- Split the old catch-all punctuation scope into block, bracket, parenthesis,
  comma, colon, and accessor-dot scopes.
- Added grammar smoke coverage for each punctuation family so bracket and
  punctuation coloring can become more theme-specific.
- Updated the editor token-scope guide with the punctuation scope families.

## Batch 102: Field And Binding Scope Split

Status: implemented after variable/property highlighting review.

- Split `name:` field declarations from `name = ...` runtime bindings in the
  TextMate fallback grammar.
- Moved workflow option-key matching before generic declarations so option keys
  such as `args =` do not get claimed as runtime local definitions first.
- Extended grammar smoke coverage for schema field captures and runtime binding
  captures, including multiline capture matching for line-leading regexes.
- Updated the editor token-scope guide with field/property/definition scope
  intent.

## Batch 103: Core Semantic Role Regression Pins

Status: implemented after LSP semantic role review.

- Added LSP snapshot regression coverage for core symbol roles: `const`
  readonly variables, schema fields as properties, `args` fields as parameters,
  function parameters, and function-local bindings.
- Documented the core semantic role expectations in the editor token-scope
  guide so TextMate fallback scopes and compiler-backed semantic tokens stay
  aligned.

## Batch 104: Imported Namespace Semantic Fallback

Status: implemented after imported-symbol highlighting review.

- Added a VS Code semantic fallback mapping for `namespace.imported` so imported
  EngLang module namespaces keep a stable scope even when a theme does not
  define imported namespace colors directly.
- Extended `ide-check` to require the `namespace.imported` fallback mapping.
- Added LSP snapshot regression coverage for `import eng.table` as a namespace
  token with `declaration` and `imported` modifiers.
- Updated the editor token-scope guide with the imported namespace contract.

## Batch 105: Deprecated And Reserved Modifier Pins

Status: implemented after deprecated/internal/planned modifier review.

- Added LSP snapshot regression coverage so deprecated `script` and `struct`
  keywords continue to carry the `deprecated` semantic modifier.
- Documented that bundled stdlib domain namespaces carry
  `defaultLibrary` + `internal` and that `planned` remains reserved until a
  source-visible planned symbol path exists.
- Kept the planned modifier visible in the editor contract, but require future
  source emission to land with LSP regression coverage and VS Code fallback
  mapping in the same change.

## Batch 106: Diagnostic Underline Range Pins

Status: implemented after diagnostic underline review.

- Added LSP JSON range regression coverage for unit mismatch, component unknown
  signal, where-local escape, unsupported `script`/`struct Args`, and direct
  uncertain comparison diagnostics.
- Reworded component-equation signal diagnostics so the unknown or unconnected
  signal is the first backticked source token, letting VS Code underline the
  actionable symbol instead of the whole equation.
- Reordered `E-UNC-DIRECT-COMPARE` message payloads so the uncertain expression
  is the first backticked token even when it appears on the right side of the
  comparison.
- Sorted unknown component-equation signal labels for deterministic diagnostic
  wording and stable editor snapshots.

## Batch 107: Side-Effect Risk Decoration Coverage

Status: implemented after side-effect decoration coverage review.

- Added review-risk records for CSV exports, `write` outputs, HTTP requests, and
  downloads so the default VS Code `eng-cli` diagnostics backend has explicit
  risk lines for those side-effect/external-boundary operations.
- Extended LSP semantic risk tokens so snapshot/live-buffer editor data marks
  `export`, `write`, `download`, and HTTP request lines with `riskHigh` or
  `riskMedium`, feeding the existing VS Code gutter and overview-ruler
  decorations.
- Added compiler review JSON regression coverage for export/write and
  HTTP/download risks, plus LSP semantic-token regression coverage for the same
  decoration-driving modifiers.
- Updated the editor token-scope contract to note that side-effect operations
  can carry both `sideEffect` coloring and risk modifiers.

## Batch 108: Internal Symbol Underline Decorations

Status: implemented after planned/internal decoration review.

- Added VS Code token-range dotted underline decorations for semantic tokens
  carrying `internal` or `planned`, separate from line-level review-risk
  markers.
- Wired the decoration refresh into diagnostics, active-editor, semantic-token,
  and semantic-token debug snapshot paths so bundled stdlib namespace internals
  such as `eng.std.domains.*` remain visibly distinct.
- Extended `ide-check` contract coverage for the new semantic symbol decoration
  helpers.
- Documented that `planned` still has no source-visible emission path today, but
  will use the same underline decoration path when compiler/LSP emits it.

## Batch 109: Coverage Result Formatting In Native Workflows

Status: implemented after workflow 01/02/03 native smoke review.

- Fixed compiler format-expression typing for `CoverageResult` fields such as
  `coverage.status`, `coverage.actual_count`, `coverage.missing_count`, and
  `coverage.max_gap_hours` so native workflow summaries can write these values
  directly without Python-side preprocessing.
- Added regression coverage for `write text` and `print` interpolation of
  coverage fields, including duration unit formatting for `max_gap_hours`.
- Re-ran the native workflow smoke path; workflow 01/02/03 source files contain
  no Python/process markers and their saved run contracts require
  `process_count = 0`.

## Batch 110: Format Unit Grammar Parity

Status: implemented after TextMate unit parity review.

- Aligned string interpolation format-unit highlighting with normal unit
  highlighting for `people/m2` and `W/m^2`.
- Added grammar smoke fixture coverage so units used inside
  `{value: .N unit}` format expressions do not silently fall back while the same
  unit highlights elsewhere.
- Regenerated the VS Code TextMate grammar from the source grammar and verified
  the updated 149-token grammar smoke contract.

## Batch 111: Unit Scope Registry Parity Guard

Status: implemented after grammar smoke coverage review.

- Strengthened the VS Code grammar smoke harness so `constant.other.unit.englang`
  and `constant.other.unit.format.englang` must both match every ASCII compiler
  unit symbol from `crates/eng_compiler/src/units.rs`.
- Closed the test gap that allowed a unit to exist in the normal unit regex but
  be absent from the format-string unit regex.
- Verified the updated grammar smoke contract still passes with 149 explicit
  token expectations plus registry-scope parity checks.

## Batch 112: Public Type Scope Registry Parity Guard

Status: implemented after completion/type highlighting review.

- Strengthened the VS Code grammar smoke harness so `support.type.englang` must
  match every base public type exported through the LSP completion seed.
- Added explicit grammar fixture coverage for workflow-facing public types such
  as `Url`, `Secret`, `DbConnection`, `DbTableRef`, `ModelCard`,
  `ModelArtifact`, `Table`, `Optional`, `TimeSeries`, and `ProcessResult`.
- Verified the updated grammar smoke contract with 159 explicit token
  expectations plus public-type scope parity checks.

## Batch 113: Workflow Option Scope Registry Parity Guard

Status: implemented after completion/with-option highlighting review.

- Extended the grammar smoke scope matcher so it can verify regexes that depend
  on surrounding syntax, such as option keys followed by `=`.
- Strengthened `variable.parameter.property.englang` coverage so every workflow
  option exported through the LSP completion seed must be highlighted as a
  with-block option key.
- Added quantity-kind scope parity to the existing public-type guard so compiler
  quantity completions and TextMate type highlighting cannot drift silently.

## Batch 114: Workflow Builtin Scope Registry Parity Guard

Status: implemented after builtin highlighting review.

- Strengthened grammar smoke so `support.function.builtin.englang` must match
  every LSP workflow builtin keyword.
- Closed the remaining TextMate test gap where a helper could remain in the LSP
  completion/semantic token surface but lose builtin fallback coloring.
- Verified the guard with the existing 159 explicit token expectations.

## Batch 115: Direct Last-Run Artifact Commands

Status: implemented after VS Code artifact command review.

- Added direct VS Code commands for opening `build/result/run_lock.json` and
  `build/result/test_results.json`.
- Kept the artifact picker unchanged while making the two remaining configured
  artifacts accessible from the Command Palette without an intermediate pick.
- Extended `ide-check` command/title coverage and README command-surface docs
  for the new artifact actions.

## Batch 116: Workflow Module Status Alignment

Status: implemented after stdlib workflow registry review.

- Promoted `eng.workflow` from planned registry wording to native workflow
  support because current runs already emit `static_run_plan.json`,
  `run_plan.json`, `run_lock.json`, `output_manifest.json`, and `run_log.json`.
- Added a `stdlib/eng/workflow.eng` boundary note so the module has the same
  explicit current/planned/review contract shape as other native workflow
  modules.
- Added a compiler registry regression check so `eng.workflow` cannot drift
  back to `planned` while the runtime artifact surface remains implemented.

## Batch 117: Completion Keyword Coloring Guard

Status: implemented after VS Code keyword-highlight consistency review.

- Strengthened the grammar smoke harness so every LSP completion keyword must
  match an accepted TextMate fallback scope in representative source context.
- Covered declaration keywords, block keywords, workflow phrases, operators,
  constants, builtins, and with-block option words instead of only checking
  that each label appears somewhere in the grammar source.
- This closes the gap where a keyword could remain discoverable through LSP
  completion but silently lose TextMate coloring.

## Batch 118: Workflow Artifact Docs Alignment

Status: implemented after workflow module status cleanup.

- Updated user-facing workflow docs to list `static_run_plan.json`,
  `run_plan.json`, and `run_lock.json` as expected native workflow artifacts.
- Added `eng.workflow` to the shared workflow contract in the composite
  workflow guide so the promoted module status is visible outside the registry
  table.
- Made the uncertain sensor workflow doc state the same zero-process contract
  as workflows 01 and 02.

## Batch 119: Static Run Plan Artifact Command

Status: implemented after VS Code workflow artifact command review.

- Added `build/result/static_run_plan.json` to the VS Code last-run artifact
  picker.
- Added the direct `EngLang: Open Last Static Run Plan` command so users can
  inspect the pre-execution workflow graph without going through the picker.
- Extended the VS Code extension contract check and README artifact list so the
  static plan command stays aligned with the promoted `eng.workflow` surface.

## Batch 120: Report And Plot Artifact Commands

Status: implemented after VS Code artifact surface review.

- Added last-run artifact picker entries for `result.engres`,
  `report_spec.json`, `plots/plot_spec.json`, `plots/plot_manifest.json`, and
  `plots/timeseries.svg`.
- Added direct Command Palette actions for opening each result/report/plot
  artifact without locating the file manually under `build/result`.
- Extended `ide-check` command/title coverage and the VS Code README artifact
  list so the editor surface mirrors the runtime artifact set.

## Batch 121: Manifest-Based Output Picker

Status: implemented after VS Code last-run output review.

- Added `EngLang: Open Last Generated Output...`, which reads
  `build/result/output_manifest.json` and lists the files actually recorded by
  the last run instead of relying only on the fixed standard artifact list.
- The picker resolves manifest paths against `build/result` or the workspace
  build directory as appropriate, filters out missing files, and opens HTML/SVG
  artifacts externally while keeping JSON, CSV, text, and `.engres` artifacts
  in VS Code.
- Extended `ide-check` command/title coverage and README docs so the new
  manifest-driven artifact surface stays visible in local VS Code installs.

## Batch 122: Module Status Display Cleanup

Status: implemented after Native IDE wording review.

- Stopped showing raw registry keys such as `native_preview` and
  `compiler_runtime_builtin` in the Native IDE Modules table.
- Updated the VS Code review panel workflow-module table to show the same
  user-facing backing labels, such as `Compiler/runtime` and
  `No executable backing`.
- Extended `ide-check` coverage so both editor surfaces keep using display
  helpers instead of leaking raw module registry status/backing keys.

## Batch 123: Workflow Module Backing Wording

Status: implemented after workflow-module docs review.

- Updated the workflow-module docs generator so the Backing column uses
  `Compiler/runtime`, `No executable backing`, and `Internal` instead of raw
  registry keys.
- Regenerated `docs/current/workflow_modules.md` to remove
  `compiler_runtime_builtin`, `none`, and `internal` backing keys from the
  public module map table.
- Kept the raw registry values in `stdlib/eng/modules.toml` as machine-owned
  source data while presenting user-facing wording in generated docs.

## Batch 124: Semantic Token Debug Readability

Status: implemented after Highlight panel review.

- Added a Text column to the Native IDE Highlight Tokens table so users can see
  which source lexeme each semantic token covers without first jumping back to
  the editor.
- Added `token_counts_by_modifier` to the VS Code `EngLang: Inspect Highlight
  Tokens` JSON output, making unit, quantity, side-effect, workflow-step, and
  risk modifier coverage easier to audit.
- Extended `ide-check` contracts so both debug surfaces keep exposing the
  richer semantic-token inspection data.

## Batch 125: Native IDE Semantic Modifier Style Coverage

Status: implemented after Native IDE highlight consistency review.

- Added explicit Native IDE highlight styles for every semantic token modifier
  exposed by the LSP legend, including declaration, imported/default-library,
  planned/internal, workflow state/input/model/DB/cache, and uncertainty
  modifiers.
- Extended `ide-check` to compare `SEMANTIC_TOKEN_MODIFIERS` from
  `eng_lsp` against `.hl-mod-*` CSS classes, so future modifier additions
  cannot silently render as plain text in the Native IDE highlight overlay.

## Batch 126: Native Workflow Inspector Fixture Cleanup

Status: implemented after workflow native-surface audit.

- Removed legacy `python run.py` commands from Native IDE case-manifest
  inspector test payloads and replaced them with native template
  materialization commands.
- Replaced the normalized review cockpit external-boundary fixture target from
  `python` to the workflow 01-style weather HTTP fixture boundary.
- Extended `ide-check` so Native IDE backend fixtures cannot reintroduce those
  Python workflow markers while workflow 01/02/03 remain native-only.

## Batch 127: VS Code Artifact Command Wording Cleanup

Status: implemented after command-palette wording review.

- Renamed user-visible VS Code review commands from `Review JSON` to
  `Review Data`, keeping command IDs stable while avoiding format-first labels.
- Renamed the last-run output-manifest command and artifact-picker label to
  `Output List`, matching the task users perform when opening generated files.
- Renamed `Process Results` in the command surface to `External Process
  Results` so native workflows with `process_count = 0` do not imply Python or
  mandatory process execution.
- Extended `ide-check` to reject the older wording in package and extension
  sources.

## Batch 128: Native IDE Effects Wording Cleanup

Status: implemented after Native IDE artifact wording review.

- Renamed the Effects panel subprocess artifact subsection from `Process
  Results` to `External Process Results`, matching the VS Code command surface
  and making it clearer that native workflows can have zero external processes.
- Extended `ide-check` so the old Native IDE panel title cannot return.

## Batch 129: Native IDE Highlight Token Filtering

Status: implemented after Highlight panel usability review.

- Added a Native IDE Highlight panel filter for semantic token text, type,
  modifier, and source line, making it easier to audit whether specific
  workflow roles such as `db`, `cache`, `workflowStep`, or `riskHigh` are
  actually present.
- Updated token type/modifier counts to reflect the current filter and added a
  shown/total badge so dense token lists remain scannable.
- Extended `ide-check` contract coverage for the new highlight-token filter
  wiring.

## Batch 130: VS Code Highlight Token Debug Samples

Status: implemented after VS Code highlight-debug usability review.

- Added `token_samples_by_type` and `token_samples_by_modifier` to
  `EngLang: Inspect Highlight Tokens`, giving representative source text,
  line, range, type, and modifiers for each semantic token group.
- Updated the VS Code README and `ide-check` contract so token debug output
  stays useful for auditing inconsistent highlighting.

## Batch 131: Native IDE Caret Token Actions

Status: implemented after caret-insight workflow review.

- Added caret insight actions for the current semantic token: `Select` chooses
  the exact token source range and `Highlight` opens the Highlight panel.
- Reused the existing byte-aware token range selection path, so multibyte source
  text keeps the same selection behavior as the Highlight token table.
- Extended `ide-check` contract coverage for the new caret-token action wiring.

## Batch 132: VS Code Artifact Label Wording Cleanup

Status: implemented after last-run artifact wording review.

- Renamed user-visible VS Code artifact picker labels and command titles from
  implementation-first names such as `Report Spec`, `Run Lock`,
  `Cache Manifest`, and `Plot Manifest` to task-oriented names such as
  `Report Data`, `Run Reproducibility Lock`, `Cache Records`, and
  `Plot Output List`.
- Kept command IDs and artifact paths stable while improving command-palette
  and quick-pick wording.
- Extended `ide-check` to reject the older internal artifact labels in package
  and extension sources.

## Batch 133: Native IDE Inspector Wording Cleanup

Status: implemented after Native IDE inspector wording review.

- Reworded Native IDE DB, model, case, and run-history labels away from
  internal data-shape names such as `Manifest`, `Spec`, and `Artifact Root`.
- Changed the visible UI vocabulary to task-oriented labels such as
  `Write Records`, `Training Plans`, `Prediction Runs`, `Case Runs`, and
  `Output Root`.
- Extended `ide-check` so those Native IDE panel labels and empty states cannot
  regress to implementation-first wording.

## Batch 134: Workflow 01 Offline Response Wording Cleanup

Status: implemented after workflow 01 native-workflow wording review.

- Renamed workflow 01's public `api_fixture` argument to `api_response_file`
  while keeping the native `http get ... with { fixture = ... }` offline
  response hook intact.
- Reworded workflow 01 prints, logs, README text, and expected review summary
  from fixture-first descriptions to pinned offline API response descriptions.
- Extended `workflows-test` so workflow 01 source/docs cannot reintroduce
  `api_fixture`, `Weather fixture`, or generic network/cache fixture wording
  while the existing Python/process guards remain in force.

## Batch 135: CLI Run Artifact Label Cleanup

Status: implemented after CLI saved-artifact output wording review.

- Replaced `eng run` artifact summary labels such as `reportspec`,
  `staticplan`, `runplan`, `runlock`, `process`, `cache`, `plotspec`, and
  `plotmanifest` with task-oriented labels such as `report data`,
  `static run graph`, `run graph`, `reproducibility lock`,
  `external process results`, `cache records`, `plot data`, and
  `plot output list`.
- Kept artifact file paths and file names stable while making saved and
  in-memory CLI summaries match the VS Code and Native IDE wording.
- Updated the `eng run` reference page and extended `workflows-test` so the
  old CLI summary labels cannot return.

## Batch 136: Editor Completion Offline Response Wording Cleanup

Status: implemented after VS Code/LSP completion wording review.

- Reworded `eng-lsp` completion details for the `fixture` option and
  HTTP-response member fields from fixture-backed wording to pinned offline
  response wording.
- Updated `eng.net` and `eng.cache` module purpose text so generated VS Code
  completion metadata describes pinned offline HTTP boundaries and cache
  records instead of network fixture/cache-manifest implementation wording.
- Regenerated VS Code editor metadata and added `eng_lsp` tests that pin the
  new completion details for `fixture`, `eng.net`, `eng.cache`, and
  `response.body`.

## Batch 137: VS Code Network Option Grammar Coverage

Status: implemented after reviewing native workflow syntax highlighting gaps.

- Added HTTP workflow fixture coverage for `fixture`, `cache`, and `cache_key`
  options inside an `http get ... with { ... }` block.
- Pinned those public network/cache options in the VS Code grammar token smoke
  test so they keep the same property-option coloring as `expected_sha256`,
  `body_size_limit`, and `response_body_limit`.

## Batch 138: Native IDE Semantic Overlay Type Coverage

Status: implemented after comparing the native IDE overlay CSS with the LSP
semantic token legend.

- Added native IDE overlay styles for `variable`, `parameter`, `modifier`, and
  `operator` semantic token types so those tokens no longer fall back to the
  same base editor color.
- Extended `ide-check` so every LSP semantic token type and modifier must have
  a corresponding native IDE overlay CSS class before editor checks pass.

## Batch 139: Native Workflow Python-Guard Generalization

Status: implemented after auditing workflow 01/02/03 native sources for Python
and legacy process calls.

- Changed `workflows-test` to discover every `examples/workflows/*/main.eng`
  file instead of checking a hard-coded workflow list.
- Kept the existing guard that rejects `run command`, Python/notebook markers,
  and legacy `select_first_row(...)` in native workflow sources, so newly added
  workflows cannot bypass the native-only execution policy.

## Batch 140: Workflow Option Highlight Completion Coverage

Status: implemented after comparing native workflow option usage with editor
grammar and LSP completion lists.

- Added `on_none`, `on_many`, and `sensor_std` to LSP workflow option
  completions and context-aware `with { ... }` suggestions.
- Added TextMate option highlighting coverage and grammar smoke expectations
  for `require_one` failure-policy options and TimeSeries `sensor_std`.
- Marked `on_none`/`on_many` semantic tokens as validation-related and
  `sensor_std`/`confidence_band` as uncertainty-related for richer IDE panels.

## Batch 141: Solver And Side-Effect Option Editor Coverage

Status: implemented after comparing public solver/write options with editor
grammar and LSP completion lists.

- Added public solver options such as `initial_derivative`,
  `initial_algebraic`, `mass_matrix`, `residual_scales`, and
  `variable_scales` to LSP option completions and context-aware `solve` /
  `simulate` with-block suggestions.
- Added `overwrite` write-context suggestions and semantic modifiers for
  solver, side-effect, and external HTTP option keys.
- Added VS Code grammar fixture coverage for solver options so public option
  keys no longer highlight inconsistently across `simulate`, `solve`, and
  write-style workflow blocks.

## Batch 142: Legacy Table Helper Completion Cleanup

Status: implemented after reviewing native workflow API wording exposure.

- Removed the legacy `select_first_row(...)` helper from LSP/static editor
  completions so new workflow authoring is guided toward `filter` +
  `require_one`.
- Kept legacy recognition for existing files, but marked `select_first_row`
  semantic tokens as deprecated instead of presenting it as a current
  `eng.table` helper.
- Regenerated editor completion metadata so VS Code and the native IDE share
  the same public completion surface.

## Batch 143: Eng Test Workflow Discovery Guard

Status: implemented after comparing `dev.bat workflows-test` with `eng test`.

- Changed `eng test` workflow example checks to discover
  `examples/workflows/*/main.eng` instead of keeping a second hard-coded
  workflow list.
- Reused the same discovered workflow list for the native source guard that
  rejects Python markers, `run command`, and legacy `select_first_row(...)`.
- Kept the minimum count check so the existing workflow 01/02/03 smoke surface
  cannot silently disappear.

## Batch 144: Native Derived Table Materialization

Status: implemented after reviewing workflow 02's surrogate-data path.

- Added runtime materialization for accepted `derive <table> column ...`
  transforms so derived numeric columns become real `RuntimeTable` columns that
  can feed filters, model training, CSV export, and SQLite writes.
- Changed workflow 02 from sampling result metrics directly to sampling input
  designs and computing `annual_electricity`, `annual_cooling`,
  `peak_cooling`, and `unmet_hours` through native derive expressions.
- Added runtime coverage for `sample lhs -> derive -> derive ->
  regression_table(...)` so the workflow cannot regress to metadata-only
  derived-column records.
- Added SQLite `mode = replace` support and changed workflow 02 DB writes to
  replace target tables, keeping repeat saved-runs clean when schemas evolve.

## Batch 145: HTTP Query Option Clarity

Status: implemented after reviewing workflow API wording and highlighting
consistency.

- Changed workflow 01 to use the explicit `query = { ... }` HTTP option map
  instead of exposing request query parameters as top-level `with` options.
- Added TextMate grammar coverage for `query`, `headers`, and `values` option
  maps so custom keys inside those maps are highlighted as properties instead
  of plain workflow bindings.
- Extended grammar fixture expectations for custom HTTP query keys.

## Batch 146: Workflow Docs Wording Cleanup

Status: implemented after reviewing public workflow/module docs for
implementation-stage wording.

- Reworded public status docs from implementation `seed` language to
  implementation tracks, while leaving deterministic sampling `seed` terms
  intact where they name the actual language option.
- Replaced workflow-module `preview` and fixture-first wording with review
  records, pinned offline response boundaries, and deterministic generation
  settings.
- Updated workflow guide pages to describe pinned inputs and workflow examples
  instead of generic fixtures.

## Batch 147: Network Option Semantic Coloring

Status: implemented after comparing TextMate option-map highlighting with LSP
semantic overlays.

- Marked HTTP/network option keys such as `query`, `headers`, `body`,
  `fixture`, `expected_sha256`, response fields, retry/timeout, and body-size
  limits with the `external` semantic modifier.
- Extended the LSP semantic-token regression so pinned HTTP response options
  stay visibly grouped as external-boundary metadata in VS Code.

## Batch 148: Pinned Response Hash Quick Fix

Status: implemented after reviewing linter quick fixes for network boundary
diagnostics.

- Added an LSP-owned quick fix for `E-NET-HASH-MISMATCH` diagnostics that
  rewrites `expected_sha256` to the actual pinned offline response hash when
  the compiler diagnostic includes the fixture hash.
- Mirrored the same quick fix in the VS Code local fallback path so snapshot
  and fallback diagnostics expose the same repair.
- Extended LSP stdio quick-fix coverage and VS Code extension contract checks
  for the new network hash action.

## Batch 149: VS Code Module Status Wording Split

Status: implemented while continuing the VS Code extension split checklist.

- Moved workflow module status/backing wording helpers out of `extension.js`
  into `moduleStatus.js`.
- Extended the VS Code extension contract so module status labels such as
  `Native workflow support`, `Compiler/runtime`, and `No executable backing`
  stay in the dedicated wording module instead of drifting back into the
  extension entrypoint.
- Synced the new module into local VS Code extension copies so the installed
  extension uses the same wording helpers as the repository source.

## Batch 150: Query Parameter Semantic Coloring

Status: implemented after checking TextMate query-map highlighting against LSP
semantic overlays.

- Marked custom HTTP/download `with` option keys as `external` semantic tokens
  when the owner line is a native network request or download.
- Added regression coverage for `query = { station = ... }` so request
  parameter keys do not lose external-boundary coloring under semantic
  highlighting.

## Batch 151: VS Code Local Quick Fix Split

Status: implemented while continuing the VS Code extension split checklist.

- Moved the JavaScript fallback quick-fix provider out of `extension.js` into
  `localCodeActions.js`.
- Kept `diagnosticCode` shared between LSP action matching and fallback quick
  fixes through the new module.
- Extended `ide-check` contract coverage so local quick-fix tokens and portable
  packaging include the split module.

## Batch 152: Workflow Boundary Phrase Highlighting

Status: implemented to reduce inconsistent first-render TextMate coloring for
native workflow boundary statements.

- Added dedicated TextMate phrase scopes for `run command ...` and
  `download ... to ...`.
- Kept nested string/function/number punctuation highlighting active inside the
  new phrase scopes.
- Added grammar smoke expectations for both phrase scopes so future keyword
  changes do not silently fall back to one-word highlighting.

## Batch 153: Explicit Native Case Materialization

Status: implemented to move workflow 02 from implicit case records toward an
explicit native `eng.case` source surface.

- Added compiler/runtime support for `materialize cases <table>` as a real
  `Table[Case]` binding with case IDs, case directories, status, failure
  reasons, and sample row hashes.
- Updated workflow 02 to bind `cases = materialize cases training_results` and
  report the case row count without Python or external process calls.
- Extended workflow smoke and docs so the explicit `CaseTable` binding cannot
  regress back to implicit-only case metadata.

## Batch 154: Editor Token Scope Contract

Status: implemented to keep VS Code highlighting taxonomy aligned with grammar
and semantic fallback changes.

- Documented every current `meta.workflow.*.englang` phrase scope, including
  process/network boundary phrases such as `run command` and `download ... to`.
- Documented the TextMate fallback scopes referenced by VS Code semantic token
  mappings so theme-facing scope names are no longer implicit in `package.json`.
- Extended `ide-check` so missing workflow phrase scopes, semantic modifiers,
  or semantic fallback scopes in `docs/internal/editor/token_scopes.md` fail the
  VS Code extension contract check.

## Batch 155: Native Case Template Apply

Status: implemented to replace row-by-row workflow 02 template rendering with a
native apply-over-cases workflow step.

- Added compiler, semantic, and runtime support for
  `case_inputs = apply case_input_template over cases` as `Table[CaseOutput]`.
- Rendered per-case input files from the original materialized case source rows
  without spawning Python or external processes, while still emitting
  `case_input` artifacts and render manifests for every case.
- Updated workflow 02, smoke expectations, and workflow docs so the native case
  input generation contract is exposed through `case_inputs.rows`.
- Added LSP with-block option completion labels for `apply` template/output
  settings so editor guidance matches the new workflow API surface.

## Batch 156: Canonical Apply Highlighting

Status: implemented to make lowered/native workflow apply calls color
consistently in VS Code before semantic tokens arrive.

- Added a dedicated TextMate phrase scope for
  `apply(<step>, over=<table>)`, matching the canonical expression form exposed
  by compiler lowering and runtime result data.
- Added grammar fixture coverage for `apply(case_input_template, over=cases)`
  so the workflow step name and case-table reference stay visibly scoped.
- Documented the new scope and the local-binding reference scope used inside
  workflow phrases in `docs/internal/editor/token_scopes.md`.

## Batch 157: Template Option Completion Parity

Status: implemented to align apply/render template with-block completions with
the compiler/runtime option contract.

- Added `template` to the LSP workflow option detail registry so
  `apply ... over ...` with-block completions no longer silently drop the
  template source option.
- Added `missing` and `artifact_kind` to render-template with-block completions,
  matching the options already accepted by semantic analysis and runtime.
- Added TextMate option-key highlighting for `template =` and grammar fixture
  coverage on the native case-template apply form.
- Added LSP regression tests for apply and render-template with-block option
  completions.

## Batch 158: Command Apply Source Highlighting

Status: implemented to keep command-style `apply` highlighting consistent with
the canonical `apply(..., over=...)` form.

- Extended `meta.workflow.apply-step.englang` to cover
  `apply <step> over <table>` instead of stopping at `over`.
- Scoped the source table reference as `variable.other.local.englang`, matching
  the local-binding treatment used by canonical apply calls.
- Added grammar fixture expectations for the full command-style apply phrase and
  table reference.

## Batch 159: Workflow Reference Phrase Highlighting

Status: implemented to keep native workflow source/reference bindings visibly
scoped inside first-render TextMate phrase highlighting.

- Extended `materialize cases <table>` so the source table is part of
  `meta.workflow.materialize-cases.englang`.
- Extended `predict <model> using <table>` so both model and input table
  references are scoped as local bindings inside the phrase.
- Extended `collect results <table>` so collected result bindings are included
  in `meta.workflow.collect-results.englang`.
- Added grammar expectations for these full phrases and their local-binding
  references.

## Batch 160: Local VS Code Extension Install Command

Status: implemented to make local VS Code linting/highlighting installation a
repeatable source-checkout workflow.

- Added `.\dev.bat vscode-package` to build release `eng.exe`/`eng-lsp.exe` and
  write `dist\local-vscode\tools\englang-vscode-<version>.vsix`.
- Added `.\dev.bat vscode-install` to package the VSIX and install it through
  the VS Code `code` CLI with `--force`.
- Updated VS Code extension and user IDE docs with the source checkout install
  path and manual VSIX fallback.

## Batch 161: VS Code Install Contract Guard

Status: implemented to keep the local VS Code packaging/install workflow from
regressing after the source-checkout installer was added.

- Extended `ide-check` to require `vscode-package` and `vscode-install` switch
  entries plus help text in `scripts/dev.ps1`.
- Extended the same contract check to require source-install instructions in
  both the VS Code extension README and native IDE how-to page.
- Kept the guard text-specific so missing install commands, hidden help output,
  or stale docs fail the existing editor contract gate.

## Batch 162: JSON Promotion Phrase Highlighting

Status: implemented to keep typed JSON promotion boundaries visibly scoped in
first-render TextMate highlighting.

- Extended `meta.workflow.promote-json-records.englang` to cover
  `promote json records <source> as <schema>` instead of stopping after
  `records`.
- Scoped the source record path as `variable.other.property.englang` and the
  target schema name as `entity.name.type.englang`.
- Added grammar expectations for the full promotion phrase and schema capture.

## Batch 163: CSV Promotion Phrase Highlighting

Status: implemented to keep CSV typed-table promotion readable in first-render
TextMate highlighting.

- Added `meta.workflow.promote-csv.englang` for
  `promote csv <source> as <schema>`.
- Scoped the source argument path as `variable.parameter.property.englang` and
  the target schema name as `entity.name.type.englang`.
- Added grammar expectations on the core workflow fixture so CSV promotion does
  not fall back to disconnected keyword coloring.

## Batch 164: JSON Object Promotion Phrase Highlighting

Status: implemented to keep native JSON object promotion aligned with the CSV
and JSON-record promotion forms in first-render TextMate highlighting.

- Added `meta.workflow.promote-json.englang` for
  `promote json <source> as <schema>`, excluding the existing
  `promote json records` table promotion form.
- Scoped local JSON payload references and target schema names inside the
  phrase so API contract promotion does not render as disconnected keywords.
- Added grammar fixture and expectation coverage for
  `promote json payload as WeatherApiPayload`.

## Batch 165: CSV Promotion Source Expression Highlighting

Status: implemented to keep CSV promotion coloring consistent across argument
paths, string paths, and `file(...)` source expressions.

- Changed `meta.workflow.promote-csv.englang` from an identifier-only match to
  a phrase scope that reaches the end of the promotion expression.
- Scoped `args.*` sources, string path sources, `file(...)` helpers, and target
  schema names inside the CSV promotion phrase.
- Reordered JSON object promotion internals so source helpers such as `file(...)`
  keep builtin highlighting before generic local-name fallback matching.
- Added grammar fixture and expectation coverage for string and `file(...)`
  CSV promotion sources.

## Batch 166: Config Promotion Phrase Highlighting

Status: implemented to align TextMate highlighting with the public JSON/TOML
config promotion completions and docs.

- Added `meta.workflow.promote-toml.englang` for
  `promote toml <source> as <schema>`.
- Extended JSON object promotion coverage with
  `promote json file(...) as WorkflowConfig`.
- Scoped config promotion target schemas through dedicated target sub-scopes so
  JSON and TOML config promotion render consistently before semantic tokens
  arrive.

## Batch 167: Read Source Expression Highlighting

Status: implemented to keep read-only I/O expressions readable across file,
argument, and dotted source references.

- Changed `meta.workflow.read-structured.englang` from a two-token match to a
  phrase scope covering the source expression.
- Scoped `file(...)`, `args.*`, and dotted response-body sources inside
  `read text/json/toml` expressions.
- Added grammar fixture and expectation coverage for
  `read json file(...)`, `read text args.notes`, `read toml args.config_toml`,
  and `read json api_response.body`.

## Batch 168: Render Template Phrase Highlighting

Status: implemented to make template rendering read as one side-effect workflow
statement in first-render TextMate highlighting.

- Added `meta.workflow.render-template.englang` for
  `render template <source>`.
- Scoped template source helpers such as `file(...)` inside the render phrase.
- Added grammar expectations for `render template file("model/base.txt")` so
  template rendering does not fall back to disconnected keyword coloring.

## Batch 169: Write And Export Target Highlighting

Status: implemented to keep explicit output artifact statements readable as
whole side-effect workflow phrases.

- Extended `meta.workflow.write-text.englang` to cover
  `write text <target>, <value>` instead of stopping after `write text`.
- Added `meta.workflow.write-json.englang` for
  `write json <target>, <value>`.
- Extended `meta.workflow.export-summary-csv.englang` to include the target CSV
  path before the summary block begins.
- Added grammar expectations for write-text payload references, write-json
  output statements, and export-summary target paths.

## Batch 170: SQLite Boundary Phrase Highlighting

Status: implemented to keep native SQLite boundary statements readable as
whole workflow phrases.

- Extended `meta.workflow.open-sqlite.englang` from `open sqlite` to
  `open sqlite <source>`.
- Extended `meta.workflow.db-write.englang` from `write <table> to <db>.table`
  to `write <table> to <db>.table("<name>")`.
- Added grammar expectations for SQLite source paths and DB table-name strings
  so workflow 02 DB boundaries do not render as disconnected fragments.

## Batch 171: HTTP Request Target Highlighting

Status: implemented to keep native network boundary requests readable as whole
workflow phrases before semantic tokens arrive.

- Extended `meta.workflow.http-request.englang` from `http <method>` to
  `http <method> <target>`.
- Scoped local and `args.*` request targets inside the HTTP phrase.
- Added grammar expectations for `http get api_url`, `http get args.api_url`,
  and `http post api_url`.

## Batch 172: Require-One Phrase Highlighting

Status: implemented to keep native table row-selection helpers readable as
workflow phrases in first-render TextMate highlighting.

- Added `meta.workflow.require-one.englang` for `require_one <table>`.
- Scoped the source table operand as a workflow-local reference.
- Added grammar expectations for `require_one designs` so filter/require-one
  table workflows do not render as isolated builtin words.

## Batch 173: Table Transform Phrase Highlighting

Status: implemented to keep native table transform helpers visibly grouped in
first-render TextMate highlighting.

- Added phrase scopes for `filter <table>`, `select <table> columns ...`,
  `sort <table> by <column> [asc|desc]`, and `join <left> with <right>`.
- Scoped table operands as workflow-local references and selected/sort columns
  as table properties.
- Added grammar fixture and expectation coverage for the table transform forms
  documented in the public table-transform surface.

## Batch 174: Coverage Target Phrase Highlighting

Status: implemented to keep TimeSeries coverage checks visibly grouped with
their checked series operand.

- Extended `meta.workflow.check-coverage.englang` from `check coverage` to
  `check coverage <series>`.
- Scoped dotted TimeSeries operands such as `measured.T_zone` as property
  references inside the phrase.
- Added grammar fixture and expectation coverage for
  `check coverage measured.T_zone`.

## Batch 175: TimeSeries Alignment Phrase Highlighting

Status: implemented to keep explicit TimeSeries alignment and resampling hooks
readable as workflow phrases.

- Added `meta.workflow.align-series.englang` for
  `align <series> with <series>`.
- Added `meta.workflow.resample-series.englang` for
  `resample <series> to <series>`.
- Scoped both source and target series operands as property references and
  pinned grammar expectations for dotted TimeSeries operands.

## Batch 176: Report Summary Phrase Highlighting

Status: implemented to keep report summary commands readable as a single
statistics workflow phrase instead of disconnected report keywords and builtins.

- Added `meta.workflow.summarize-series.englang` for
  `summarize <series> by [...]`.
- Scoped the summarized series operand as a property reference and `by` as a
  word operator.
- Kept statistic names such as `mean`, `time_weighted_mean`, `p90`, and `p95`
  on built-in function scopes inside the summary list.

## Batch 177: Plot Distribution Phrase Highlighting

Status: implemented to keep uncertainty distribution plots readable as a
single report workflow phrase.

- Added `meta.workflow.plot-distribution.englang` for
  `plot distribution(<distribution>)`.
- Kept `plot` and `distribution` on report keyword scopes while scoping the
  plotted distribution operand as a property reference.
- Added grammar expectations against the uncertainty/report fixture so the
  report plot command stays covered with the summary command.

## Batch 178: TimeSeries Integrate Phrase Highlighting

Status: implemented to make common TimeSeries integration forms read as native
workflow phrases in VS Code fallback highlighting.

- Added `meta.workflow.integrate-call.englang` for
  `integrate(<series>, over=<axis>)`.
- Added `meta.workflow.integrate-series.englang` for
  `integrate <series> over <axis>`.
- Added grammar fixture coverage for both function-style and command-style
  integration forms next to the existing TimeSeries fixture.

## Batch 179: TimeSeries Statistic Phrase Highlighting

Status: implemented to keep common TimeSeries statistic calls visibly tied to
their series operand and axis.

- Added `meta.workflow.stat-axis-call.englang` for
  `mean(<series>, axis=<axis>)`, `max(<series>, axis=<axis>)`, and related
  axis statistic calls.
- Added `meta.workflow.stat-series.englang` for command-style statistic phrases
  such as `mean <series> over <axis>`.
- Added fixture expectations for function-style and command-style statistic
  forms used by workflow 03 and the public TimeSeries syntax docs.

## Batch 180: Report Show And Plot Phrase Highlighting

Status: implemented to make the common report block output lines read as
complete report actions in first-render VS Code highlighting.

- Added `meta.workflow.plot-series.englang` for `plot <series> over <axis>`.
- Added `meta.workflow.show-report.englang` for `show <value>` report entries,
  including dotted values such as `coverage.status`.
- Added report fixture expectations so workflow 01/02/03-style `show` rows and
  workflow 03-style TimeSeries plots stay covered.

## Batch 181: Native Model Call Phrase Highlighting

Status: implemented to keep workflow 02's native model-building calls readable
as model workflow actions instead of isolated builtin names.

- Added `meta.workflow.regression-table.englang` for
  `regression_table(<table>, target=..., features=..., ...)`.
- Scoped model option names such as `target` and `features` as parameter
  properties inside the regression call.
- Added `meta.workflow.model-summary-call.englang` for `evaluate(<model>)`,
  `model_card(<model>)`, and related single-model summary calls.

## Batch 182: Runtime Message Phrase Highlighting

Status: implemented to keep direct CLI/debug output and structured runtime log
messages visibly grouped as side-effecting workflow statements.

- Added `meta.workflow.print-message.englang` for `print "..."` lines.
- Added `meta.workflow.log-message.englang` for `log <level> "..."` lines.
- Scoped `debug`, `info`, `warn`, and `error` log levels as language constants
  while preserving string interpolation scopes inside message strings.

## Batch 183: Distribution Call Phrase Highlighting

Status: implemented to keep sampling ranges and uncertainty distributions
readable as value-distribution calls in VS Code fallback highlighting.

- Added `meta.workflow.distribution-call.englang` for `uniform(...)`,
  `normal(...)`, `measured(...)`, and `interval(...)`.
- Scoped distribution option keys such as `mean`, `std`, `error`, and
  `samples` as parameter properties inside the call.
- Added grammar expectations for workflow 02 sampling ranges and workflow 03
  uncertainty distribution calls.

## Batch 184: Where Block Highlighting

Status: implemented to make `where { ... }` condition blocks render like other
first-class EngLang blocks in VS Code fallback highlighting.

- Added `where` to the TextMate block opener scope alongside `args`, `with`,
  `report`, and related block keywords.
- Added grammar fixture coverage for a table `filter` followed by a `where`
  condition block, matching workflow 01/02 and command-style examples.

## Batch 185: VS Code Block Indentation Rules

Status: implemented to improve editing ergonomics for EngLang block syntax in
the local VS Code extension.

- Added VS Code language-configuration indentation rules so lines ending in
  `{` increase indentation and standalone `}` decreases indentation.
- Extended `ide-check` to require the indentation contract alongside the
  existing `#` comment and `///` continuation checks.
- Documented `language-configuration.json` as an editor source of truth.

## Batch 186: Summary CSV Field Phrase Highlighting

Status: implemented to make durable summary CSV field declarations readable as
artifact field rows instead of disconnected identifiers and keywords.

- Added `meta.workflow.summary-field.englang` for
  `<value> as <unit> with "<format>"` rows inside summary export blocks.
- Scoped the exported value as a property, `as`/`with` as word operators, and
  the display unit, including dimensionless `1`, as a unit token.
- Added grammar expectations for the side-effect/export fixture.

## Batch 187: Validation Phrase Highlighting

Status: implemented to make validation and test statements read as review
checks instead of isolated keywords inside ordinary expressions.

- Added `meta.workflow.validation.englang` for `validate`, `assert`, and
  `golden ... matches ...` lines.
- Kept `matches` and `within` scoped as validation keywords inside validation
  phrases.
- Added grammar expectations for assertion, golden-file, and top-level
  validation lines.

## Batch 188: Status Condition Highlighting

Status: implemented to make `on { status == ... }` checks readable as status
conditions instead of a plain identifier plus a generic comparison.

- Added `meta.workflow.status-condition.englang` for `status == <state>` and
  `status != <state>` lines.
- Scoped `status` as an option/property, the comparison as an operator, and
  the lifecycle state as a language constant.
- Added grammar expectations for the report/solver fixture's `on` block.

## Batch 189: Method Self And Return Highlighting

Status: implemented to make class method bodies and function returns less
visually flat in fallback highlighting.

- Added `variable.language.self.englang` for `self` references in class
  methods, keeping member properties such as `.name` distinct.
- Added `meta.workflow.return-statement.englang` so `return <value>` lines
  highlight as a full statement instead of only a lone keyword.
- Added grammar expectations for `self` and `return Q` in the declaration
  fixture.

## Batch 190: Typed Binding Declaration Highlighting

Status: implemented to keep typed value declarations visually distinct from
schema/class field declarations.

- Added `meta.declaration.typed-binding.englang` for `name: Type = ...`
  declarations.
- Scoped typed binding names as `variable.other.definition.englang` while
  leaving `name:` field declarations on the existing field scope.
- Added grammar expectations for URL/secret typed bindings and a TimeSeries
  typed binding.

## Batch 191: Function Parameter Highlighting

Status: implemented to make function and method signatures readable in the
fallback grammar when semantic tokens have not arrived yet.

- Added `meta.declaration.parameter.englang` for function/method parameter
  declaration fragments.
- Scoped parameter names as `variable.parameter.function.englang`, with the
  declared parameter type kept on `support.type.englang`.
- Added VS Code semantic fallback mappings for `parameter.declaration` and
  quantity-bearing parameters.
- Added grammar expectations for `m_dot` and `dT` in the declaration fixture.

## Batch 192: User Function Call Highlighting

Status: implemented to make user-defined function calls visible before
semantic tokens resolve symbols.

- Added `entity.name.function.call.englang` for identifiers followed by `(`.
- Kept built-in helper scopes earlier in the grammar so public built-ins such
  as `file(...)`, `uniform(...)`, and `integrate(...)` keep their existing
  highlighting.
- Added grammar fixture coverage for `coil_heat(...)`.

## Batch 193: Function Scoped Reference Semantic Tokens

Status: implemented to make function bodies stay role-aware after declaration
highlighting.

- Added compiler-backed semantic tokens for function parameter references in
  function bodies, not just parameter declarations.
- Added compiler-backed semantic tokens for function-local references such as
  `return Q`, preserving local declaration tokens.
- Added LSP regression coverage so declaration tokens keep `declaration` while
  body references are emitted as separate scoped tokens.

## Batch 194: Table Workflow Operand Semantic Tokens

Status: implemented to make native table workflow sources and columns visible
as workflow-step references in semantic highlighting.

- Added `workflowStep` semantic modifiers to sample and table-transform result
  bindings.
- Added semantic tokens for table transform source tables, secondary join
  tables, selected/sorted/derived columns, predicate columns, and join keys.
- Added VS Code fallback mapping for `variable.workflowStep` so table workflow
  bindings and source operands are theme-compatible.

## Batch 195: Model Workflow Operand Semantic Tokens

Status: implemented to make native model workflow calls show their input and
feature roles, not just the model result binding.

- Added compiler-backed semantic tokens for ML/model source operands such as
  `evaluate(surrogate)`, `model_card(surrogate)`, and
  `predict surrogate using designs`.
- Added semantic tokens for `regression_table` target and feature column
  operands.
- Added LSP regression coverage so model declarations and repeated model/table
  references are counted as role-aware semantic tokens.

## Batch 196: Legacy Seed Status Alias Cleanup

Status: implemented to remove a stale module-status alias after the registry
moved to native workflow support wording.

- Removed the unused legacy seed-status alias from compiler module-status
  labeling.
- Removed the same alias from Native IDE and VS Code module-status display
  helpers.
- Kept `native_preview` rendering as the user-facing `Native workflow support`
  status.

## Batch 197: VS Code Problems Source Setting

Status: implemented to remove implementation-facing backend wording from the
primary VS Code linting configuration.

- Added `englang.problemsSource` with user-facing `file` and `live` values for
  choosing how the Problems panel updates.
- Kept `englang.diagnosticsBackend` as a deprecated compatibility alias so
  existing workspaces that set the old values continue to work.
- Updated extension output, README, and user IDE guide wording so users see
  file/live checks instead of backend implementation names.
- Extended `ide-check` contract coverage so the new setting cannot disappear
  and legacy wording does not return to the primary user path.

## Batch 198: Native HTTP Request Method Boundaries

Status: implemented to align the public HTTP request vocabulary with native
network boundary analysis and editor highlighting.

- Generalized compiler network request analysis from `http get` only to
  `http get/post/put/patch/head/request/fetch`.
- Kept the offline fixture/reproducibility policy unchanged while recording
  the actual request method in review, run-plan, run-log, result, and output
  manifest artifacts.
- Updated LSP semantic tokens, document symbols, and with-block completion
  owner detection so non-GET HTTP requests keep external-boundary colors and
  network options.
- Added compiler, runtime, and LSP regressions for non-GET request methods.

## Batch 199: Native Standard Text Writer

Status: implemented to move workflow 01 standard-file generation out of
string-shaped `write text` and into a native table artifact writer.

- Added `write standard_text <table>` with `with { output = ... }` support,
  table-only compiler validation, output-path diagnostics, and generated-output
  path policy checks.
- Implemented deterministic runtime table-text materialization with schema,
  row-count, column, source-hash, and TSV body metadata; output manifest records
  it as `standard_file` by default.
- Updated workflow 01 to write `standard_weather_file.txt` through
  `write standard_text weather` while keeping Python/external process count at
  zero.
- Added TextMate grammar, LSP completion/semantic token, compiler, runtime,
  workflow-doc, and stdlib registry coverage for the new public writer.

## Batch 200: Workflow Option Highlighting Consistency

Status: implemented to reduce VS Code completion/highlighting gaps for public
workflow `with` options.

- Added missing LSP workflow-option completions for implemented or contextual
  options: `transaction`, `epochs`, and process `cache_ttl`.
- Made with-option semantic token modifiers owner-aware so DB, model, sample,
  case, template, report, solver, cache, and external-boundary options get
  consistent semantic coloring.
- Updated DB write, process, and model-training with-block completion tests, plus
  semantic token coverage for DB transaction/key, model epochs, generated output,
  case materialization, and report plot options.
- Regenerated VS Code editor metadata and TextMate grammar after adding
  `cache_ttl` to the grammar option vocabulary.

## Batch 201: Workflow DB Target Path Resolution

Status: implemented to make workflow 02's SQLite output argument real instead of
display-only.

- Updated workflow 02 so `args.database_target` feeds the SQLite connection
  path and both DB writes record explicit `transaction = commit`.
- Fixed runtime path evaluation so `file(args.<name>)` and `dir(args.<name>)`
  resolve the arg value before output and DB manifest paths are materialized.
- Added runtime regression coverage for SQLite connections using an args-backed
  path and verified generated manifests no longer use the literal
  `args.database_target`.
- Cleaned workflow 02 expected docs to reflect eight generated case inputs and
  the args-backed SQLite boundary.

## Batch 202: Live HTTP Runtime Boundary

Status: implemented for native live `http://` GET/download execution while
preserving pinned fixture/cache workflows.

- Allowed fixture-less `response.body` JSON sources to compile as runtime-bound
  sources and resolve once the runtime materializes the HTTP response body.
- Added runtime live `http://` request/download execution with retry, timeout,
  body-size limit, SHA-256 verification, cache hit replay, and
  review/run-log/output manifest integration.
- Added regression coverage for live HTTP response-body JSON promotion and
  cache replay without Python or `run command`.
- Initially kept live `https://` behind an explicit local-build TLS diagnostic;
  Batch 295 replaces that placeholder with native TLS-backed HTTP(S) execution.

## Batch 203: Live HTTP Hash Quick Fix

Status: implemented for VS Code/LSP linter quick-fix parity between pinned
fixture and live HTTP hash mismatch diagnostics.

- Extended `E-NET-HASH-MISMATCH` quick-fix parsing so `expected_sha256` can be
  updated from both fixture SHA-256 diagnostics and live HTTP `observed` hash
  diagnostics.
- Added LSP stdio regression coverage for the live HTTP observed-hash message
  shape while keeping the existing fixture-backed quick fix.

## Batch 204: Stdlib Module Definition Navigation

Status: implemented for LSP-backed go-to-definition on bundled stdlib module
imports.

- Routed `textDocument/definition` and `--definition-stdin` requests on
  `use eng.<module>` symbols to `stdlib/eng/<module>.eng` when a module source
  file exists.
- Kept a registry fallback to `stdlib/eng/modules.toml` for modules that are
  documented in the registry but do not have a dedicated `.eng` source file.
- Added an LSP stdio regression test for `use eng.net` navigation to the
  bundled `stdlib/eng/net.eng` module declaration.

## Batch 205: LSP Workspace Symbol Search

Status: implemented for persistent LSP clients that need workspace-wide symbol
lookup.

- Advertised `workspaceSymbolProvider` from the stdio LSP server.
- Added `workspace/symbol` handling over open documents plus `.eng` files under
  initialized workspace roots, with bounded scanning and skipped generated/tool
  directories.
- Added an LSP stdio regression test that finds an unopened `.eng` file's
  schema symbol through `rootUri` workspace search.

## Batch 206: VS Code Workspace Symbol Bridge

Status: implemented for VS Code users invoking workspace-wide symbol search.

- Added `eng-lsp --workspace-symbols <workspace-root> [query]` so short-bridge
  editor clients can reuse the same bounded `.eng` workspace symbol search as
  persistent LSP clients.
- Registered a VS Code `WorkspaceSymbolProvider` that searches each open
  workspace folder and converts LSP workspace symbols into VS Code symbol
  results.
- Extended `ide-check` contract coverage and LSP stdio tests so the bridge,
  provider wiring, and CLI command stay present.

## Batch 207: Model Split Option Completion Parity

Status: implemented after comparing TextMate option coloring with LSP editor
metadata.

- Added `split` to the LSP workflow option completion registry so model
  evaluation and leakage-lint contexts no longer color the option without also
  offering it through completion.
- Added position-aware with-block completion coverage for `evaluate(model)` so
  `split`, cache, and cache-key options stay available from the editor.
- Extended `ide-check` metadata guards so generated VS Code completion seed
  output cannot drop `split` while the grammar still highlights it.

## Batch 208: Scoped Workflow Option Highlighting

Status: implemented to reduce VS Code option-coloring drift and false-positive
workflow option scopes.

- Scoped TextMate workflow option coloring to `with { ... }` blocks so ordinary
  top-level bindings such as HTTP response field names do not get option-key
  fallback colors.
- Split function named arguments such as `std=`, `samples=`, `scale=`, and
  `offset=` into a function-parameter scope instead of treating them as workflow
  options.
- Aligned the workflow option regex with the LSP completion registry, removed
  unsupported HTTP response-field/body labels from option completions, and added
  compiler-known solver/display options such as `variable_scale`,
  `display_unit`, `backend`, and `consistency_tolerance`.
- Added a reverse grammar regression guard so TextMate cannot highlight a
  workflow option label unless the LSP completion registry exposes the same
  label.

## Batch 209: Native HTTP Request Body Option

Status: implemented to close a visible API gap for non-GET network requests.

- Added compiler/runtime support for `http post`, `http put`, and `http patch`
  request bodies through `with { body = ... }`.
- Live `http://` requests now send the native body value and record
  `body_sha256` in network boundary metadata instead of treating `body` as a
  query parameter or unsupported editor-only option.
- Reintroduced `body` in LSP completions and scoped TextMate option coloring
  now that the compiler/runtime path accepts it.
- Added compiler/runtime regressions for body lowering, body cache identity,
  live POST body transmission, and body hash reporting.

## Batch 210: Plot Unit Axis Completion Accuracy

Status: implemented to remove an invalid IDE suggestion from plot workflows.

- Replaced the plot `with { ... }` completion for invalid `unit =` with the
  compiler-supported `unit y =` and `unit x =` axis options.
- Kept the TextMate `unit y =`/`unit x =` special scope while removing `unit =`
  from the generic workflow option highlighter.
- Taught assigned-option tracking to recognize spaced unit-axis keys, so an
  existing `unit y = ...` no longer reappears as a duplicate completion.

## Batch 211: Native Workflow Snippet Starters

Status: implemented to make VS Code starter snippets match the native workflow
surface used by the shipped examples.

- Added snippets for native `http get`, `http post` bodies, deterministic
  `sample lhs`, case materialization/template application, regression
  prediction tables, SQLite table writes, and `write standard_text` artifacts.
- Updated the plot snippet to show compiler-supported `unit y =` axis options
  instead of leaving axis configuration implicit.
- Extended the VS Code extension contract check so these native workflow
  snippet starters and their key tokens are guarded by `ide-check`.

## Batch 212: Behavior Wrapper Typed Output Contracts

Status: implemented to replace a behavior-node seed placeholder with static
typed contract data that the compiler/report already knows.

- Static report behavior nodes now derive predictor and external adapter output
  quantity/unit contracts from their resolved input signal, matching the
  deterministic identity-wrapper runtime behavior.
- Runtime behavior graph integration now labels those output contracts with
  runtime wording instead of `*_seed` wording.
- Extended report, CLI smoke, and native IDE smoke regressions so predictor and
  external behavior nodes must expose typed output contracts.

## Batch 213: Plot Unit Option Quick Fix

Status: implemented to reduce migration friction after removing the invalid
plot `unit =` option from completions/highlighting.

- Added an LSP-owned quick fix for `E-WITH-OPTION-001` when the unknown option
  is exactly `unit = ...`, replacing the key with the compiler-supported
  `unit y = ...` form.
- Mirrored the same quick fix in the VS Code local fallback provider for cases
  where LSP code actions are unavailable.
- Extended LSP stdio code-action regressions and the VS Code extension
  contract so this quick fix remains covered.

## Batch 214: Native Workflow Semantic Token Color Parity

Status: implemented to make compiler-backed semantic highlighting closer to
the TextMate grammar for native workflow phrases.

- Marked native workflow keywords such as `read`, table transforms,
  `column`/`columns`, `results`, and model `using` with role-specific semantic
  modifiers instead of leaving them as generic keywords.
- Classified table helpers such as `filter`, `select`, `sort`, `derive`, and
  `require_one` as workflow-step built-ins, with context-aware table `join`
  handling that does not recolor path helper calls.
- Marked `Table[Case]` and `Table[CaseOutput]` bindings as workflow-step
  values, and extended LSP semantic-token regressions plus generated VS Code
  editor metadata.

## Batch 215: VS Code Model Keyword Scope Fallback

Status: implemented to keep VS Code theme fallback mapping aligned with the
new model keyword semantic-token combinations.

- Added a `keyword.model` semantic token scope mapping so model connective
  tokens such as `using` can inherit the same rich fallback scopes as model
  helper functions when semantic highlighting is enabled.
- Extended the VS Code extension contract so future model keyword semantic
  tokens cannot ship without a corresponding fallback scope mapping.

## Batch 216: Semantic Scope Coverage Guard

Status: implemented after fixture-based semantic-token coverage found several
VS Code fallback scope gaps.

- Added an `eng_lsp` regression test that snapshots the VS Code grammar
  fixtures and verifies every emitted custom semantic modifier pair has a
  corresponding `semanticTokenScopes` fallback mapping in the extension
  package.
- Filled missing mappings for quantity/type tokens, axis tokens,
  property-level side-effect/external/solver tokens, and keyword/class/property
  review-risk tokens.
- Strengthened `dev.bat lsp-check` so the full `eng_lsp` test suite, including
  this cross-editor scope coverage guard, runs before LSP smoke checks.

## Batch 217: Editor User Wording Cleanup

Status: implemented after reviewing VS Code and Native IDE first-user docs for
internal editor-service wording.

- Reworded the VS Code extension README around live checks, hover, completion,
  formatting, and go-to-definition so the public path explains user behavior
  instead of backend process shape.
- Reworded the Native IDE how-to VS Code section so it describes shared
  compiler-backed editor data without presenting the LSP/snapshot status as a
  user-facing limitation.
- Added an `ide-check` guard for the most confusing internal editor-service
  phrases in user-facing editor docs.

## Batch 218: Native IDE Highlight Panel Wording

Status: implemented after finding semantic-token implementation labels in the
Native IDE Highlight panel.

- Renamed Highlight panel badges, filters, section titles, table headings, and
  empty states from semantic-token/type/modifier wording to highlight
  categories and details.
- Renamed the advanced raw data toggle from `Raw semantic token JSON` to `Raw
  highlight data`.
- Extended `ide-check` so the most confusing semantic-token UI phrases cannot
  reappear in the Native IDE.

## Batch 219: VS Code Highlight Inspection Wording

Status: implemented after reviewing VS Code setting descriptions and the
Inspect Highlight Tokens output.

- Reworded `englang.lspPath` and semantic highlighting setting descriptions so
  they describe live editor features and role-aware colors instead of
  editor-service or semantic-token internals.
- Added user-facing `highlight_*` fields to the Inspect Highlight Tokens JSON
  while keeping the existing semantic-token keys for compatibility.
- Reworded the missing-data warning and README description around highlight
  inspection, and extended `ide-check` to guard the new public wording.

## Batch 220: Current Architecture Seed Wording Cleanup

Status: implemented after auditing current architecture docs for stale
implementation-seed wording.

- Replaced `native VM seed`, `bytecode VM seed`, and unit-registry seed wording
  with current native runtime and registry terms.
- Updated runtime artifact examples from stale `sampled_seed` and
  `trained_seed` statuses to current `uncertainty_attached` and
  `trained_linear` artifact vocabulary.
- Reworded current data-boundary docs from missing policy seeds to
  missing-value policies.

## Batch 221: Hide Deprecated VS Code Diagnostics Setting

Status: implemented after reviewing VS Code Settings exposure for internal
backend names.

- Removed deprecated `englang.diagnosticsBackend` from the VS Code settings
  contribution so new users only see `englang.problemsSource` with `file` and
  `live` values.
- Kept the extension's legacy `diagnosticsBackend` read path so older
  workspaces that already have `eng-cli` or `lsp-snapshot` in `settings.json`
  continue to behave correctly.
- Updated the VS Code extension contract so the deprecated setting stays
  code-compatible but is not re-exposed in the Settings UI.

## Batch 222: Native IDE Advanced Data Wording

Status: implemented after reviewing Native IDE empty states and advanced data
toggles for raw JSON/artifact wording.

- Reworded Quality, Kernel Plan, Workflow, Effects, and Network/Cache empty
  states from artifact-data phrasing to user task records/results.
- Renamed closed raw JSON toggles across review, quality, kernel, workflow
  node, effects, network/cache, DB, model, and case panels to `Advanced ... data`.
- Reworded the user guide `eng-lsp.exe --smoke` paragraph away from
  editor-service snapshot terminology and extended `ide-check` to guard the
  old UI labels.

## Batch 223: VS Code Live Editor Output Wording

Status: implemented after reviewing VS Code Output panel messages emitted by
live editor checks, completion, and definition lookups.

- Reworded live-buffer check failures from LSP snapshot phrasing to `Live
  editor check failed`.
- Reworded completion and definition subprocess failures to task-oriented
  lookup/data messages.
- Extended the VS Code extension contract so old snapshot-worded Output panel
  messages cannot return while the internal CLI bridge names remain available
  for compatibility.

## Batch 224: Package And Status Wording Cleanup

Status: implemented after reviewing user guide, package guide text, asset
inventory text, and current status docs for stale implementation wording.

- Reworded package guide text from editor-service and language-seed phrasing
  to editor tooling checks and standard library source files.
- Reworded current status docs from implementation-seed and smoke/snapshot
  phrasing to implementation tracks and editor-request checks.
- Extended `ide-check` so the editor documentation wording guard also covers
  current status docs and blocks stale implementation wording there.

## Batch 225: String Interpolation Scope Coverage

Status: implemented after checking TextMate string interpolation coverage
against the syntax-highlighting TODO.

- Added TextMate scopes for double-quoted string delimiters so themes can color
  string boundaries separately from string contents.
- Pinned interpolation braces and format separators in the grammar smoke
  expectations alongside interpolation variables, precision, and unit-format
  scopes.
- Regenerated the packaged TextMate grammar from the source grammar.

## Batch 226: VS Code Dotted Symbol Word Pattern

Status: implemented after reviewing VS Code language configuration for editor
selection/navigation ergonomics.

- Added a VS Code `wordPattern` so dotted EngLang symbols such as
  `args.input`, `sensor.T_supply`, and `eng.net` are treated as single word
  units by editor actions.
- Extended `ide-check` so the language configuration must keep dotted symbol
  word selection coverage.

## Batch 227: Native Workflow Output Wording Guard

Status: implemented after rechecking workflow 01/02/03 sources and expected
docs for stale Python/process assumptions.

- Reworded workflow 02's expected output-manifest summary so native model and
  prediction records are not described as files produced by an external
  process.
- Extended `workflows-test` to scan workflow public docs for that stale
  external-process output phrase, alongside the existing source guards that
  reject Python markers, `run command`, legacy `select_first_row(...)`, and
  nonzero process execution.

## Batch 228: Advanced Editor Metadata CLI Wording

Status: implemented after reviewing public CLI reference docs for internal
snapshot-first wording.

- Renamed the CLI index entry from `LSP snapshot` to `Advanced editor metadata
  JSON`.
- Reframed the snapshot reference as a maintainer/editor-tooling contract, not
  a first-user command page.
- Reworded the `eng-lsp.exe` CLI spec from snapshot-path phrasing to advanced
  editor metadata extraction while preserving the real `--snapshot` command and
  `eng-lsp-snapshot-v1` format names.

## Batch 229: Review Fingerprint UI Wording

Status: implemented after checking VS Code and Native IDE review panels for
internal ReviewDocument field labels.

- Renamed the visible `Semantic Hash` section in the VS Code review panel to
  `Review Fingerprint`.
- Renamed the same Native IDE Review panel label to `Review Fingerprint`.
- Extended `ide-check` so both editor surfaces keep the user-facing label and
  do not reintroduce the internal semantic-hash wording.

## Batch 230: Current Status Editor Wording

Status: implemented after checking current status, tracks, and Native IDE
how-to docs for stale editor-service and smoke/snapshot wording.

- Reworded `docs/current/status.md` so packaged editor tooling is described as
  smoke checks plus metadata JSON, not an LSP smoke/snapshot binary.
- Renamed the IDE track heading and status detail from `IDE / LSP` and
  `LSP smoke/snapshot tooling` to editor tooling and metadata smoke checks.
- Reworded the Native IDE guide roadmap from persistent LSP integration to
  long-running editor integration.
- Extended `ide-check` editor-doc wording guards to cover `status.md` and
  `tracks.md`.

## Batch 231: Generic Type TextMate Scopes

Status: implemented after reviewing generic type highlighting for class/object
and workflow artifact declarations.

- Added a `meta.type.generic.englang` TextMate scope for bracketed generic
  type expressions such as `Table[T]`, `Optional[DirectoryPath]`, and
  `TimeSeries[Time]`.
- Added a `variable.parameter.type.englang` scope for generic type arguments so
  themes can distinguish container types from inner type parameters.
- Pinned grammar expectations for the public declaration fixture and documented
  the new scopes in the editor token-scope contract.

## Batch 232: Generic Type Completion Scope Guard

Status: implemented after checking how generic public type completions can drift
from TextMate fallback highlighting.

- Strengthened the VS Code grammar smoke harness so every public generic type
  label exported by LSP completion metadata must match
  `meta.type.generic.englang`.
- Kept the existing base-type guard for `support.type.englang`, while deriving
  it from the same completion label list as the generic guard.

## Batch 233: Generic Type Semantic Token Overlay

Status: implemented after finding that LSP lexical semantic token scanning used
full generic completion labels while source scanning works word-by-word.

- Changed the LSP semantic scanner to derive public type base names from generic
  completion labels, so `Secret`, `Table`, `Optional`, and `TimeSeries` receive
  type semantic tokens in source.
- Added bracket-aware generic type scanning so inner type arguments such as `T`
  in `Table[T]` receive type semantic tokens without globally coloring every
  standalone `T`.
- Added VS Code semantic fallback mapping for the broad `type` selector and
  connected it to generic TextMate scopes.
- Extended editor contract checks so the broad type semantic fallback mapping
  remains present.

## Batch 234: Generic Parameter Signature Highlighting

Status: implemented after reviewing function and method parameter declaration
captures for generic type syntax.

- Split TextMate parameter declaration type captures so `Table[T]` in a
  function signature colors the container type, brackets, and inner type
  argument separately.
- Added a grammar fixture function with a generic table parameter and pinned
  the function name, parameter name, container type, and type argument scopes.

## Batch 235: Builtin Call Scope Guard

Status: implemented after finding that the generic user-function call pattern
could still match public builtin helpers before the fallback builtin pattern in
some TextMate traversal paths.

- Excluded public workflow builtins such as `file(...)`, `normal(...)`,
  `mean(...)`, and `integrate(...)` from the generic user-function call pattern.
- Strengthened the VS Code grammar smoke harness so builtin helper labels from
  the LSP registry cannot be captured as `entity.name.function.call.englang`.

## Batch 236: Confidence Band Option Quick Fix

Status: implemented after reviewing `E-WITH-OPTION-001` quick fixes for plot
and report option naming mistakes.

- Added an LSP-owned quick fix that renames `confidence = ...` to
  `confidence_band = ...` for unknown with-option diagnostics.
- Mirrored the same rename in the VS Code local quick-fix fallback.
- Extended stdio and `--code-actions-stdin` quick-fix tests so the repair stays
  available for persistent and short-lived editor flows.
- Extended the VS Code extension contract so the local fallback keeps the
  confidence-band repair alongside the existing `unit =` to `unit y =` repair.

## Batch 237: Workflow Completion Wording Cleanup

Status: implemented after reviewing public workflow completion details for
implementation-oriented module wording.

- Reworded table, case, model, fixture, and `require_one` option completion
  details so the editor explains the user action instead of exposing helper
  implementation names.
- Regenerated VS Code editor metadata so completion fallback text matches the
  LSP source of truth.
- Extended LSP completion tests so `require_one`, `regression_table`, `predict`,
  and `fixture` details stay action-oriented.

## Batch 238: Reference Skeleton Cleanup

Status: implemented after auditing public reference docs for placeholder pages.

- Replaced language reference skeleton pages for types, units, schemas, report
  language, and diagnostics with short current lookup material and links to the
  detailed source-of-truth pages.
- Replaced the diagnostics reference skeleton with a stable index and diagnostic
  wording contract.
- Renamed the language index section from planned skeletons to focused lookup
  pages so public docs no longer advertise unfinished placeholders.

## Batch 239: SQLite Transaction Value Highlighting

Status: implemented after checking VS Code snippets against TextMate and LSP
keyword coverage.

- Added `commit` and `rollback` to the LSP keyword/completion vocabulary with
  DB semantic-token modifiers.
- Added the same values to the TextMate constant fallback so
  `transaction = commit` and `transaction = rollback` are colored consistently
  in VS Code before semantic tokens arrive.
- Extended grammar fixtures and semantic-token tests to cover both transaction
  values.

## Batch 240: Operator Keyword Semantic Parity

Status: implemented after comparing TextMate workflow/operator keywords against
the LSP keyword vocabulary.

- Added `else`, `of`, and `vs` to the LSP keyword/completion vocabulary so
  semantic highlighting no longer falls behind TextMate fallback coloring.
- Extended semantic-token tests for conditional, typed TimeSeries, and plot
  comparison phrases that already had TextMate coverage.
- Regenerated editor completion metadata so VS Code and Native IDE completion
  fallback data includes the same keyword surface.

## Batch 241: TextMate Keyword Completion Guard

Status: implemented after finding `model`, `output`, and `on` still colored by
TextMate fallback but absent from the LSP keyword/completion vocabulary.

- Added `model`, `output`, and `on` to the LSP keyword/completion vocabulary so
  workflow model phrases, system output declarations, and join predicate blocks
  receive consistent semantic coloring and completion fallback.
- Extended semantic-token tests with actual `output` and `on { ... }` source
  examples instead of only checking registry membership.
- Added an LSP regression test that reads the VS Code TextMate grammar and
  fails when word-like `keyword.*` fallback scopes are missing from LSP keyword
  completions.
- Preserved the more specific `output` workflow-option completion detail when
  the same label is also present as a declaration keyword.

## Batch 242: Behavior Preview Status Display Labels

Status: implemented after auditing internal component behavior artifacts for
raw pending-integration status strings leaking into user-facing report and IDE
tables.

- Kept report-spec JSON contract statuses stable, but mapped component behavior
  and solver-preview HTML cells to readable labels such as "Predictor contract
  not connected to this language-level solve".
- Applied the same display mapping in the Native IDE assembly and component
  behavior inspectors.
- Updated smoke expectations so report HTML verifies readable labels while
  raw contract status remains in JSON artifacts.

## Batch 243: State-Space Type Highlighting Parity

Status: implemented after comparing advanced solver examples against the
VS Code/LSP public type vocabulary.

- Added state-space declaration keywords `states`, `inputs`, `outputs`, and
  `operator` to LSP completion and solver semantic-token classification.
- Added public editor completions and TextMate fallback scopes for
  `StateVector[T]`, `InputVector[T]`, `OutputVector[T]`, `Derivative[T]`,
  `LinearOperator[From -> To]`, plus report/plot file path types.
- Extended semantic-token scanning so nested type expressions such as
  `LinearOperator[RoomState -> Derivative[RoomState]]` color their inner type
  identifiers instead of only the outer base name.
- Added VS Code grammar fixture coverage for state-space declarations and
  generic solver type names.

## Batch 244: Public Preview And Fixture Wording Cleanup

Status: implemented after scanning public README and scope documents for
product-facing `preview`, `seed`, and generic `fixture` wording.

- Renamed the VS Code extension README heading and opening sentence so the
  extension is described as supported editor tooling, not a preview object.
- Reworded the root README's solver-heavy repository note from smoke fixtures
  to advanced/internal validation examples.
- Replaced public scope references to implementation seeds and generic
  fixtures with implementation tracks, validation examples, or narrower test
  discovery wording.

## Batch 245: Network Boundary Wording Cleanup

Status: implemented after reviewing public network/cache docs for fixture-first
wording that could obscure the pinned offline response contract.

- Reworded side-effect policy and stdlib module descriptions from
  offline/fixture phrasing to pinned offline/cache HTTP boundary records.
- Updated the `E-NET-UNPINNED-REPRO` CLI summary to say pinned response file
  instead of the lower-level fixture term.
- Replaced remaining public artifact/language references to internal fixtures
  with internal examples where no `fixture = ...` option was being named.

## Batch 246: Sampling Alias Highlighting Parity

Status: implemented after checking sampling workflow aliases that were accepted
by the compiler but not fully aligned in LSP/editor metadata.

- Added `latin-hypercube` to LSP workflow builtin completions alongside
  `latin_hypercube`.
- Extended semantic token scanning so the hyphenated sampling alias is colored
  as one default-library workflow function instead of being split by the
  identifier scanner.
- Updated TextMate builtin fallback, generated editor metadata, and grammar
  smoke expectations for the alias.

## Batch 247: Typed SQLite Target Argument

Status: implemented after reviewing workflow 02's public API surface for path
arguments that were still exposed as plain strings.

- Changed workflow 02's `database_target` argument from `String` to `FilePath`
  and opened SQLite with `open sqlite args.database_target`.
- Updated runtime regression coverage so typed path arguments feed SQLite
  connection paths without wrapping the arg in `file(...)`.
- Kept output/review artifact paths stable while removing an unnecessary
  conversion from the visible workflow code.

## Batch 248: Native Workflow Claim Wording

Status: implemented after reviewing public workflow docs for weak or
implementation-oriented wording.

- Reworded the workflow examples README to state that workflows 01/02/03 run
  without Python or external processes.
- Replaced artifact reference wording about Jacobian sparsity placeholders with
  Jacobian sparsity metadata.

## Batch 249: Fill Missing Highlighting Parity

Status: implemented after comparing public TimeSeries fill syntax against the
editor vocabulary.

- Added `fill` and `fill missing` to LSP workflow builtin completions and
  semantic token modifiers.
- Added TextMate phrase coverage for `fill missing <table>.<column>` so the
  command no longer falls back to disconnected identifiers.
- Extended grammar fixture and editor metadata coverage for the public
  TimeSeries fill workflow.

## Batch 250: Schema Quality Semantic Tokens

Status: implemented after checking workflow 01/03 schema quality blocks
against the LSP semantic token overlay.

- Marked schema `constraints` and `missing` quality keywords with the
  validation semantic modifier.
- Added semantic validation coverage for constraint words such as `between` and
  `monotonic`, plus missing-policy tokens such as `interpolate`, `max_gap`, and
  `error`.
- Extended LSP snapshot coverage so schema quality blocks stay as colorful and
  meaningful as workflow validation lines.

## Batch 251: Zero-Process Run Output Wording

Status: implemented after workflow smoke output still printed an external
process label for native workflows with `process_count = 0`.

- Changed CLI artifact summaries to label zero-process runs as
  `process results (0 external processes)` instead of implying an external
  process ran.
- Kept the existing `external process results` label for workflows that do run
  `run command`.
- Updated the `eng run` reference output and added unit coverage for the
  process-result label selection.

## Batch 252: Native Surrogate Workflow Wording

Status: implemented after reviewing workflow 02 labels for wording that still
made the native surrogate example sound like it executed an external simulator.

- Reworded workflow 02 public titles from external-simulation-first wording to
  native surrogate wording while keeping the existing path stable.
- Clarified that external simulator adapters can feed the same contracts later,
  but the current executable workflow has zero external process adapters.
- Updated smoke failure text and expected artifact summaries to avoid
  external-process or opaque-tool implications.

## Batch 253: Public Seed Wording Cleanup

Status: implemented after scanning current public docs for seed wording that
did not describe deterministic sampling.

- Reworded TimeSeries axis provenance from an axis seed to an axis source.
- Reworded the completion philosophy rule from implementation seeds to
  implementation evidence.
- Left deterministic sampling `seed` wording unchanged where it is the public
  reproducibility API.

## Batch 254: Schema Quality TextMate Parity

Status: implemented after comparing schema quality blocks in workflows 01/03
against the immediate VS Code TextMate scopes.

- Split `constraints` and `missing` block openers out of the generic block
  scope and into the validation keyword scope.
- Added TextMate validation coverage for schema quality words such as
  `interpolate` and `monotonic` so pre-LSP coloring matches the semantic token
  intent more closely.
- Extended grammar fixtures and smoke expectations for schema quality blocks.

## Batch 255: Stdlib Reference Current-Scope Cleanup

Status: implemented after checking public docs for stale standard-library
index wording.

- Replaced the `docs/reference/stdlib/index.md` early/planned-only index with a
  current lookup page tied to `stdlib/eng/modules.toml`.
- Clarified that `docs/user/README.md` is contributor-facing source guidance
  and that `docs/user/index.md` is the user documentation home.
- Extended `docs-check` with a stdlib-reference wording guard so stale early
  index text and missing native workflow module references fail locally.

## Batch 256: Registry-Aware Stdlib Import Diagnostics

Status: implemented after finding that VS Code/LSP could navigate
`use eng.<module>` while compiler linting still treated the same lines as file
import errors.

- Changed compiler import resolution so registered `eng.*` stdlib module
  imports are handled through `stdlib/eng/modules.toml` instead of the file
  import resolver.
- Added warnings for planned/internal stdlib module imports and a specific
  unknown-module error for misspelled `eng.*` imports.
- Documented stdlib module import behavior in the functions/imports reference
  and CLI diagnostic code list.

## Batch 257: Stdlib Import Quick Fix

Status: implemented after adding registry-aware stdlib import diagnostics.

- Added an LSP quick fix for misspelled `eng.*` module imports that replaces
  close matches such as `eng.nte` with the registry-backed module name.
- Kept candidate lookup in `eng_lsp` through the compiler-owned module
  registry instead of adding a JavaScript module-name table.
- Extended VS Code/IDE contract checks and LSP stdio coverage for the new
  quick fix path.

## Batch 258: Stdlib Import Status Semantic Tokens

Status: implemented after turning `eng.*` module imports into registry-aware
diagnostics and quick fixes.

- Marked source-visible registered `eng.*` module imports as `namespace`
  semantic tokens with `defaultLibrary`, `declaration`, and `imported`.
- Added `planned` and `internal` modifiers for planned/internal stdlib imports
  such as `use eng.stats` and `use eng.system`.
- Added VS Code fallback mapping and contract coverage for `namespace.planned`
  so planned module imports remain visible even without semantic-theme support.

## Batch 259: VS Code Quick Fix Fallback Merge

Status: implemented after reviewing the VS Code linter quick-fix bridge.

- Changed the VS Code code action provider to merge LSP-owned quick fixes with
  local fallback quick fixes instead of hiding local fixes whenever any LSP
  action is available.
- Deduplicated merged quick fixes by title, kind, and edit fingerprint so LSP
  actions still stay first without showing duplicate repairs.
- Extended extension contract coverage and README wording so partial LSP bridge
  responses cannot silently remove JavaScript fallback repairs.

## Batch 260: Workflow 02 Native Wording Guard

Status: implemented after rechecking workflow 02 public wording against the
native source and zero-process smoke contract.

- Reworded workflow 02 docs from external-simulator-first phrasing to a native
  sampling, case, model, prediction, and DB workflow description.
- Kept the existing path stable while making future simulator adapters a
  layered possibility, not the current executable behavior.
- Extended `workflows-test` so public workflow docs cannot reintroduce stale
  external-process wording that makes the native workflow look Python/process
  backed.

## Batch 261: VS Code Code Action Provider Split

Status: implemented after the quick-fix merge path made code-action
orchestration nontrivial enough to own separately.

- Moved VS Code code-action orchestration from `extension.js` into
  `codeActionProvider.js`.
- Kept local quick-fix generation and LSP code-action conversion in their
  existing focused modules while the provider owns LSP-first fallback merging.
- Extended extension and portable-package contract checks so the split provider
  remains packaged and does not drift back into the entrypoint.

## Batch 262: VS Code Semantic Token Provider Split

Status: implemented while continuing to reduce `extension.js` ownership of
editor feature providers.

- Moved VS Code semantic-token provider orchestration into
  `semanticTokensProvider.js`.
- Kept low-level LSP semantic-token conversion in `lspSemanticTokens.js`, while
  the provider owns the VS Code setting check, snapshot request, cache update,
  and planned/internal decoration refresh.
- Extended extension and portable-package contract checks so the semantic-token
  provider remains packaged and does not drift back into the entrypoint.

## Batch 263: VS Code Formatting Provider Split

Status: implemented while continuing the VS Code entrypoint split.

- Moved VS Code document-formatting provider orchestration into
  `formattingProvider.js`.
- Kept the compiler/LSP stdin formatting request in `extension.js`, while the
  provider owns document filtering and the full-document `TextEdit` conversion.
- Extended extension and portable-package contract checks so formatting helpers
  remain packaged and do not drift back into the entrypoint.

## Batch 264: VS Code Folding Provider Split

Status: implemented while continuing the VS Code entrypoint split.

- Moved VS Code folding-range provider orchestration into
  `foldingRangeProvider.js`.
- Kept shared LSP kind conversion in `lspKinds.js`, with folding-specific range
  conversion owned by the provider module.
- Extended extension and portable-package contract checks so folding helpers
  remain packaged and do not drift back into the entrypoint.

## Batch 265: VS Code Navigation Provider Split

Status: implemented while continuing the VS Code entrypoint split.

- Moved VS Code document-symbol, workspace-symbol, and definition provider
  orchestration into `navigationProviders.js`.
- Kept live compiler/LSP process calls in `extension.js`, while the provider
  module owns cancellation-aware VS Code provider methods and shared snapshot
  cache handoff.
- Extended extension and portable-package contract checks so navigation
  providers stay packaged and keep reusing `lspNavigation.js`.

## Batch 266: VS Code Model Training Highlighting

Status: implemented while improving workflow phrase highlighting consistency.

- Added `meta.workflow.model-train-call.englang` for `train_test_split(...)`,
  `train_regression(...)`, `regression(...)`, and `mlp(...)` model-training
  calls.
- Expanded the sampling/model grammar fixture and token expectations so model
  training APIs get the same phrase-level TextMate coverage as prediction and
  model-card calls.
- Updated the editor token-scope contract so the new model-training scope is
  documented and checked with the rest of the workflow phrase scopes.

## Batch 267: VS Code Model Summary Option Highlighting

Status: implemented while tightening native model API phrase highlighting.

- Expanded `meta.workflow.model-summary-call.englang` from single-argument
  matches to full parenthesized calls so `evaluate(model, split=split)` remains
  highlighted as one model workflow action.
- Added grammar fixture and token expectations for option-bearing `evaluate`
  calls and `leakage_lint(...)`.
- Updated the token-scope contract wording to document the supported optional
  split argument form.

## Batch 268: Clarify Raw Read Highlighting Surface

Status: implemented while reducing API wording/highlighting confusion.

- Removed unsupported `read csv` phrase highlighting from
  `meta.workflow.read-structured.englang`; raw reads are `read text`,
  `read json`, and `read toml`.
- Updated editor token-scope wording to point CSV table users to
  `promote csv <source> as <schema>`.
- Added a grammar smoke guard so `read csv file(...)` cannot silently become a
  highlighted raw-read workflow phrase again.

## Batch 269: Workflow 02 Native Artifact Smoke Gate

Status: implemented while tightening native workflow verification.

- Extended `workflows-test` so workflow 02 must emit native sample, case,
  model-card, prediction-manifest, and DB-manifest records, not only
  `process_count = 0`.
- Added output-manifest checks for `case_input`, template render manifests,
  SQLite database/write manifests, `model://surrogate_model`, and
  `model://predictions` artifacts.
- Kept the guard focused on generated artifacts from the public workflow smoke
  so stale external-process or opaque-tool regressions fail during dev checks.

## Batch 270: VS Code Runtime Discovery Split

Status: implemented while continuing the VS Code entrypoint split.

- Moved VS Code runtime and LSP executable discovery, workspace-root lookup,
  current-workspace lookup, and EngLang configuration lookup into
  `runtimeDiscovery.js`.
- Kept the extension entrypoint focused on command/provider registration while
  diagnostics, run/review commands, and live editor requests reuse the shared
  runtime-discovery helpers.
- Extended extension and portable-package contract checks so runtime discovery
  stays packaged and does not drift back into `extension.js`.

## Batch 271: VS Code Review Panel Renderer Split

Status: implemented while continuing the VS Code entrypoint split.

- Moved review-panel HTML rendering, source-line normalization, and last-run
  artifact link availability shaping into `reviewPanelRenderer.js`.
- Kept `extension.js` responsible for VS Code command wiring, webview message
  handling, artifact opening, source navigation, and review-risk decorations.
- Extended extension and portable-package contract checks so the review panel
  renderer stays packaged and does not drift back into `extension.js`.

## Batch 272: VS Code Artifact Opener Split

Status: implemented while continuing the VS Code entrypoint split.

- Moved last-run artifact opening, generated-output picking, and output-list
  path normalization into `artifactOpeners.js`.
- Kept `extension.js` focused on command registration while artifact commands
  use the shared artifact opener service.
- Reworded generated-output empty/error messages away from raw
  `output_manifest.json` wording and added contract checks so the helper stays
  packaged.

## Batch 273: ANN Model Alias Highlighting Parity

Status: implemented while tightening supported keyword highlighting parity.

- Added the compiler-supported `ann(...)` model-training alias to LSP keyword
  completion, semantic-token model modifiers, and with-block option completion.
- Added TextMate fallback coverage so `ann(...)` is colored like `mlp(...)`
  before semantic tokens arrive.
- Added grammar fixture/smoke expectations and editor token-scope docs so the
  alias does not drift out of VS Code highlighting again.

## Batch 274: Distribution Helper Highlighting Parity

Status: implemented while aligning compiler-supported uncertainty helpers with
VS Code highlighting and completion metadata.

- Added `distribution(...)` to LSP builtin completion and semantic-token
  uncertainty modifiers while preserving `plot distribution(...)` as report
  context.
- Extended TextMate fallback highlighting so `distribution(...)` is scoped like
  `normal(...)` and `uniform(...)`.
- Added grammar coverage for compiler-supported uncertainty option aliases such
  as `sigma`, `n`, `lower`, and `upper`.

## Batch 275: Interpolation Wording Cleanup

Status: implemented while reducing accidental Python framing in public docs.

- Reworded the formatting reference from "Python-like string interpolation" to
  "brace-based string interpolation" so the docs describe EngLang syntax
  without implying a Python runtime dependency.

## Batch 276: Function Argument Completion Context

Status: implemented while improving live LSP completion ergonomics for native
workflow helpers.

- Added position-aware property completions inside supported helper calls such
  as `distribution(...)`, `normal(...)`, `regression(...)`, `mlp(...)`, and
  `train_test_split(...)`.
- Kept member completion and with-block completion higher priority so `table.`
  fields and owner-specific `with { ... }` options keep their current behavior.
- Added regression tests for uncertainty and model argument contexts, including
  filtering already-assigned named arguments.

## Batch 277: Multi-Line Helper Argument Completion

Status: implemented while tightening native workflow helper completion in
normal formatted code.

- Extended helper-call argument completion to inspect the source prefix up to
  the cursor, so multi-line calls such as `distribution(` followed by indented
  named arguments keep the same contextual suggestions as single-line calls.
- Added regression coverage for multi-line uncertainty and model helper calls.

## Batch 278: VS Code LSP Request Split

Status: implemented while continuing the VS Code thin-client split.

- Moved live editor snapshot, completion, definition, formatting, code-action,
  and workspace-symbol subprocess calls into `lspRequests.js`.
- Kept `extension.js` focused on command/provider wiring while the LSP request
  bridge owns snapshot promise reuse, cancellation, stdin payloads, and
  user-facing live-editor error wording.
- Extended extension and portable-package contract checks so LSP request
  helpers stay packaged and do not drift back into `extension.js`.

## Batch 279: Command-Style Case Apply Native Guard

Status: implemented while tightening workflow 02 native execution coverage.

- Extended case-apply binding detection to understand both
  `apply(case_input_template, over=cases)` and the public command-style
  `apply case_input_template over cases` form directly.
- Added a runtime guard for both forms so case-output table materialization is
  not dependent on canonical lowering alone.
- Added LSP semantic and VS Code grammar smoke coverage for the workflow 02
  template-step phrase used by the native surrogate example.

## Batch 280: VS Code Command Handler Split

Status: implemented while continuing the VS Code thin-client split.

- Moved run, example-runner, execution-profile, review JSON/panel, source-line
  navigation, artifact-click, and semantic highlight debug command handlers
  into `commandHandlers.js`.
- Kept `extension.js` focused on activation, provider wiring, diagnostics, and
  decoration state while command handlers own subprocess calls and webview
  command behavior.
- Extended extension and portable-package contract checks so command handlers
  stay packaged and do not drift back into `extension.js`.

## Batch 281: VS Code Decoration Controller Split

Status: implemented while continuing the VS Code thin-client split.

- Moved review-risk line decorations and internal/planned semantic symbol
  decorations into `decorations.js`.
- Kept `extension.js` responsible for activation and event wiring while the
  decoration controller owns marker creation, refresh, hover wording, and
  review snapshot line mapping.
- Extended extension and portable-package contract checks so decoration helpers
  stay packaged and do not drift back into `extension.js`.

## Batch 282: Native IDE Lexical Highlight Fallback

Status: implemented to make editor coloring less dependent on completed
analysis snapshots.

- Added a lightweight lexical fallback highlighter for the native IDE editor
  overlay so comments, strings, numbers, units, keywords, operators,
  constants, modules, and completion-backed types/functions keep color before
  or between semantic checks.
- Kept compiler/LSP semantic tokens authoritative when they are current; the
  fallback only fills stale or token-empty source ranges.
- Extended IDE contract checks so the fallback renderer and its core styles do
  not silently regress to plain text.

## Batch 283: Native IDE Interpolation Highlight Fallback

Status: implemented to improve string readability before semantic analysis is
current.

- Split fallback string rendering so interpolation braces and interpolation
  contents are highlighted separately from plain quoted text.
- Reused the lexical fallback inside interpolation contents, so variable paths,
  format punctuation, numeric precision, and units in strings keep useful color.
- Extended IDE contract checks so interpolation-aware fallback rendering and
  styling do not regress to plain string coloring.

## Batch 284: VS Code Semantic Fallback Scope Coverage

Status: implemented to reduce theme-dependent highlight gaps in VS Code.

- Audited semantic token pairs emitted for the grammar fixture set against
  `package.json` `semanticTokenScopes`.
- Added fallback scopes for plain and standard-modifier pairs such as
  declarations, read-only values, default-library symbols, comments, numbers,
  properties, parameters, and report functions.
- Extended extension contract checks and token-scope docs so these fallback
  mappings remain covered.

## Batch 285: Semantic Fallback Pair Test Coverage

Status: implemented to keep VS Code fallback coverage from narrowing again.

- Strengthened the `eng_lsp` fixture test so it checks every emitted semantic
  token selector used by the grammar fixtures, including plain token types and
  standard modifiers.
- Removed the previous custom-modifier-only filter that missed selectors such
  as `comment`, `number`, `class.declaration`, and `variable.readonly`.

## Batch 286: Network Offline Response Wording

Status: implemented to reduce fixture-like wording on public native API paths.

- Added `offline_response` as the preferred `eng.net` option for pinned HTTP
  response files, while keeping `fixture` as a legacy alias for existing files.
- Updated workflow 01, stdlib notes, LSP completion metadata, semantic token
  fixtures, and runtime/repro/cache status wording to surface offline response
  terminology instead of test-fixture terminology.
- Kept internal compatibility for old cache records and `fixture` option usage
  so existing samples do not fail during the wording transition.

## Batch 287: Weather Offline Response Filename

Status: implemented to keep workflow 01 filenames aligned with API wording.

- Renamed the workflow 01 pinned response data file from
  `sample_api_response.json` to `offline_weather_response.json`.
- Updated the workflow argument default and side-effect policy wording so public
  docs no longer describe the network repro boundary with the legacy `fixture`
  option.

## Batch 288: Native Train Regression Surface

Status: implemented to move workflow 02 away from helper-looking model calls.

- Added `train regression <table>` as a native table-regression workflow phrase
  backed by the existing compiler `MlInfo` and runtime linear-training path.
- Let model `with` blocks supply `target`/`features`/`test`/`seed`, with
  `y`/`x` and `test_fraction` aliases for users coming from modeling notation.
- Updated workflow 02, snippets, TextMate phrase scopes, LSP completions,
  semantic tokens, and docs so the public path reads as a workflow step while
  legacy `regression_table(...)` remains supported.
- Fixed parser ambiguity where `test = ...` inside a `with` block could be
  treated as a test-block start.

## Batch 289: Native IDE Syntax Catalog Fallback

Status: implemented to reduce native IDE and VS Code first-render highlight
drift.

- Added the LSP editor `syntax_catalog` to native IDE bootstrap data so the
  native UI receives the same compiler-owned keyword, workflow helper, option,
  type, quantity, and unit lists used by the VS Code extension.
- Changed the native IDE stale-buffer lexical highlighter to build keyword,
  type, quantity, workflow option, and unit matching from that catalog instead
  of relying on UI-local token lists.
- Updated docs and native IDE regression coverage so the metadata-backed
  fallback remains part of the editor contract.

## Batch 290: Native IDE Editor Keyboard Basics

Status: implemented to make the textarea-backed native editor less dependent
on browser defaults.

- Added native IDE Ctrl+/ line-comment toggling with `#`, matching the VS Code
  language configuration comment style.
- Added Tab and Shift+Tab handling for selected blocks and the current line so
  focus no longer leaves the editor during normal indentation work.
- Added Enter auto-indent for block openers, closing-brace splits, and `///`
  documentation comment continuation, with IDE contract coverage and user docs.

## Batch 291: Native IDE Review-First Panel Order

Status: implemented to make the right sidebar match the intended review flow.

- Moved Checks, Review, Quality, Effects, Artifacts, and Run ahead of
  implementation-heavy panels such as Modules, Flow, Assembly, Kernel, Case,
  Model, and DB.
- Kept Highlight available near the review panels as an inspection/debug aid,
  while leaving Problems in the bottom panel.
- Updated user docs and `ide-check` contract coverage so future UI edits do
  not accidentally bury the primary review path behind advanced panels.

## Batch 292: Native IDE Bracket Pair Editing

Status: implemented to align native IDE textarea editing with the VS Code
language configuration basics.

- Added native IDE auto-closing and selection wrapping for `{}`, `[]`, `()`,
  and `"` pairs.
- Added closing-pair skip and empty-pair Backspace deletion so pair editing
  behaves predictably during repeated edits.
- Added `}` on-type indentation for blank indented lines, plus docs and
  `ide-check` contract coverage for the new editor helpers.

## Batch 293: Native IDE Compiler Formatter Action

Status: implemented to expose the compiler-owned source formatter in the
native editor.

- Added an `ide_format` Tauri command backed by `eng_compiler::format_source`,
  so native IDE formatting uses the same compiler formatter as CLI and VS Code.
- Added a toolbar Format action that formats the current unsaved buffer, marks
  the tab dirty when the source changes, and reports formatter-clean buffers
  without writing files.
- Aligned native editor auto-indent to the formatter's four-space indentation
  and extended Rust, docs, and `ide-check` coverage for the formatter path.

## Batch 294: Native IDE Bracket Match Insight

Status: implemented to add first-pass bracket matching feedback to the native
editor.

- Added caret-adjacent `{}`, `[]`, and `()` matching in the editor meta bar.
- Shows the matching bracket line/column when found and an unmatched marker
  when the bracket has no counterpart.
- Updated user docs and `ide-check` contract coverage for the new caret
  insight helpers.

## API And Wording Cleanup Candidates

- Continue reviewing public command names and setting text for terms that are
  too internal as new workflow APIs move from planned to supported.
- Continue reviewing stdlib/module status words where docs still expose
  implementation history instead of current support scope.
- Audit workflow helper names for readability, especially `check coverage`,
  `predict <model> using <table>`, and DB write forms.
- Keep CLI errors and setting descriptions action-oriented: what happened, why
  it matters, and the next valid command.

## Batch 295: Native HTTPS Network Execution

Status: implemented to remove the remaining live-network TLS placeholder from
the native runtime path.

- Enabled the existing `ureq` HTTP client with native TLS support.
- Removed the runtime guard that rejected fixture-less live `https://` requests
  with `E-NET-TLS-UNAVAILABLE`.
- Updated workflow/module maturity docs so `eng.net` now describes live HTTP(S)
  execution instead of HTTPS/TLS as a planned seed.

## Batch 296: VS Code Run Wording Cleanup

Status: implemented to reduce user-facing confusion between VS Code run actions
and the EngLang `run command` external-process syntax.

- Reworded the VS Code extension description from "run commands" to "program
  execution".
- Reworded the execution-profile setting description from "EngLang run
  commands" to "EngLang program runs".
- Updated the generated VSIX manifest description source to use the same
  wording.

## Batch 297: VS Code Stdlib Module Quick Fix Fallback

Status: implemented to keep stdlib import typo repairs available when the
short LSP code-action bridge has no payload.

- Passed the generated completion seed into the VS Code local quick-fix
  fallback.
- Added a local `E-STDLIB-MODULE-UNKNOWN` action that suggests the closest
  `eng.*` module from generated editor metadata, matching the existing LSP
  behavior such as `eng.nte` to `eng.net`.
- Added `ide-check` contract coverage so the fallback and completion-seed
  wiring do not regress.

## Batch 298: Semantic Highlight Constants

Status: implemented to reduce cases where TextMate colored a public workflow
constant but compiler-backed semantic tokens left it unclassified.

- Added LSP lexical semantic tokens for language constants such as log levels,
  execution profiles, cache statuses, case statuses, and solver algorithm
  labels.
- Mapped cache status constants to the `cache` modifier, case/workflow status
  constants to `workflowStep`, and solver algorithm constants to `solver`.
- Kept `distribution(...)` as an uncertainty helper while marking bare
  `distribution T` report directives as report tokens.
- Expanded semantic-token regression coverage for these constant labels.

## Batch 299: Native Workflow Artifact Contract Smoke

Status: implemented to make workflow 01/03 smoke tests prove native artifact
contracts rather than only checking that the examples run.

- Added workflow 01 `workflows-test` checks for native network boundaries,
  pinned offline-response status, cache manifest records, typed JSON promotion,
  and standard-file output manifest entries.
- Added workflow 03 `workflows-test` checks for native uncertainty propagation,
  sensor standard-deviation review records, CSV/text outputs, plot specs,
  plot manifests, and report-spec links.
- Kept the existing zero-external-process assertion across workflow 01/02/03 so
  these examples cannot silently reintroduce Python or shell process execution.

## Batch 300: Behavior Status Artifact Wording

Status: implemented to remove legacy seed-style pending-integration status
names from component behavior and solver-preview artifacts.

- Renamed behavior integration statuses to explicit pending-integration names
  such as `predictor_call_contract_pending_integration`.
- Renamed behavior contract/profile statuses to explicit metadata names so
  report and IDE artifacts no longer expose implementation-seed wording.
- Updated report, compiler, runtime, CLI smoke, native IDE, and golden artifact
  expectations together so the new status vocabulary is covered.
- Added an IDE contract guard that rejects the legacy behavior status keys in
  the UI label mapper.

## Batch 301: Uncertainty Argument Quick Fixes

Status: implemented to reduce VS Code/LSP linter friction for uncertainty
constructor mistakes used by the native uncertainty workflow path.

- Added LSP-owned quick fixes for `E-UNC-ARGS-*` diagnostics that replace
  malformed uncertainty calls with compiler-provided examples when available.
- Added targeted repairs for unsupported `distribution(kind=...)`, unsupported
  `propagate(method=...)`, and invalid `samples`/`n` values.
- Mirrored the same repairs in the VS Code local quick-fix fallback and
  extended contract coverage so LSP/local quick-fix parity does not drift.

## Batch 302: Uncertainty Call Grammar Coverage

Status: implemented to reduce VS Code TextMate highlighting gaps for
uncertainty workflow helpers beyond constructor calls.

- Extended the uncertainty-call grammar block to cover `propagate(...)`,
  `ensemble(...)`, and `probability(...)` in addition to measured, interval,
  normal, uniform, and distribution constructors.
- Added grammar fixture expectations for `propagate(...)` and `probability(...)`
  so propagation options such as `method`, `scale`, and `offset` stay colored as
  function arguments before semantic tokens arrive.
- Regenerated the packaged TextMate grammar from the source grammar.

## Batch 303: Assembly Balance Status Wording

Status: implemented to remove implementation-seed wording from component
assembly balance metadata exposed through review/report artifacts.

- Renamed `balanced_metadata_seed` to `balanced_metadata`.
- Renamed `underdetermined_seed` and `overdetermined_seed` to
  `underdetermined_metadata` and `overdetermined_metadata`.
- Added a dev contract guard so the old balance-status strings cannot return to
  compiler-generated artifact metadata.

## Seed-To-Implementation Candidates

- Cache replay/invalidation: network offline-response cache
  materialization/replay is implemented with hash checks; broader process/model
  replay and explicit invalidation remain to be designed around artifact safety.
- Live network execution: live HTTP(S) GET/download is implemented with timeout,
  retry, body limit, SHA-256 verification, and cache replay; broaden request
  body/auth policy.
- Model training surface: native `train regression <table>` now feeds the
  existing model-card, metrics, and prediction-table path; future work is
  broader algorithm coverage and clearer multi-model naming.
- Case orchestration: current case manifests are materialized through workflow
  records; a native `apply/run cases` surface needs scheduler, resume, cache,
  and failure policy.
- DB read/query support: current SQLite writes are supported; reads and query
  APIs need typed schema mapping, transaction policy, and review records.

## Native IDE Usability Candidates

- Continue improving the caret token insight with richer source range actions
  once the textarea-backed editor can expose them without fighting text
  selection.
- Continue reviewing inspector workflows for dense, repeated debugging tasks.

## VS Code Linter And Highlighting Candidates

- Promote the VS Code extension from short editor-service bridge calls to a
  persistent LSP client when the protocol surface is stable.
- Broaden cross-file go-to-definition beyond static file imports once bundled
  module symbols expose definition URI/range metadata.
- Expand compiler-backed semantic token coverage for richer workflow step
  references after those source spans become first-class metadata.
- Continue expanding snapshot coverage for grammar and completion vocabulary
  as new workflow phrases become public.
- Surface richer quick fixes for diagnostics that need broader semantic context.

## Docs Cleanup Candidates

- Remove or archive duplicate historical docs once their current claims are
  represented in `docs/current/status.md`, `docs/current/workflow_modules.md`,
  or reference docs.
- Keep public user docs short and task-oriented; move implementation history
  to `docs/internal` or `docs/archive`.
- Make `docs/README.md` the single navigation entry and avoid parallel indexes
  that disagree.
- Generated tables should name their source, especially module status and
  diagnostics catalogs.
- Delete stale command examples when the CLI no longer supports them.
