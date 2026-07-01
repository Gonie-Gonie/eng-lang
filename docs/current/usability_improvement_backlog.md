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

## API And Wording Cleanup Candidates

- Review public command names and setting text for terms that are too internal:
  `hybrid`, `native target`, and `opaque boundary`.
- Make workflow command examples consistent: prefer `eng.exe run <file.eng>
  --save-artifacts` where saved artifacts are discussed.
- Continue reviewing stdlib/module status words where docs still expose
  implementation history instead of current support scope.
- Audit workflow helper names for readability, especially
  `select_first_row(...)`, `check coverage`, `predict <model> using <table>`,
  and DB write forms.
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

- Add panel-level empty states that say which artifact is missing and which
  command produces it.
- Keep problem filtering by severity/code and source-line jump targets covered
  as part of native IDE regression checks.
- Group raw JSON inspectors behind advanced toggles so first-run users see the
  reviewable summary first.
- Add source span breadcrumbs in case/model/DB/network panels.

## VS Code Linter And Highlighting Candidates

- Promote `eng-lsp` from snapshot mode to a persistent LSP server when the
  protocol surface is stable.
- Add semantic tokens from compiler metadata so symbols, quantities, units,
  functions, and module names can be colored like a mature language extension.
- Continue expanding snapshot coverage for grammar and completion vocabulary
  as new workflow phrases become public.
- Surface richer quick fixes for diagnostics that need semantic context, such
  as missing units and unsupported `script` blocks.

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
