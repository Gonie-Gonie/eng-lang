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

- Renamed the user-facing label for `native_preview` and `supported_seed`
  registry entries from `Native preview` to `Native workflow support`.
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
- Counted `native_preview` and `supported_seed` registry entries as native in
  the native IDE category view while keeping the machine-readable status keys
  unchanged.
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
  selected `case_001` sample row so the CSV summary matches the native
  `sample lhs` table and rendered case input.
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

- Added phrase-aware TextMate scopes for `read json`/`read toml`/`read text`/
  `read csv`, `open sqlite`, and `write <table> to db.table(...)` so native
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

## API And Wording Cleanup Candidates

- Continue reviewing public command names and setting text for terms that are
  too internal as new workflow APIs move from planned to supported.
- Continue reviewing stdlib/module status words where docs still expose
  implementation history instead of current support scope.
- Audit workflow helper names for readability, especially `check coverage`,
  `predict <model> using <table>`, and DB write forms.
- Keep CLI errors and setting descriptions action-oriented: what happened, why
  it matters, and the next valid command.

## Seed-To-Implementation Candidates

- Cache replay/invalidation: network fixture cache materialization/replay is
  implemented with hash checks; broader process/model replay and explicit
  invalidation remain to be designed around artifact safety.
- Live network execution: current network support is fixture/offline-first;
  add live HTTP only with timeout, retry, body limit, secret redaction, and
  reproducible pinning policy.
- Model training surface: current native prediction table exists; broaden
  training syntax only after model-card and feature/target contracts stay
  stable across examples.
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

- Promote `eng-lsp` from snapshot mode to a persistent LSP server when the
  protocol surface is stable.
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
