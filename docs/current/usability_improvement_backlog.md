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

## API And Wording Cleanup Candidates

- Review public command names and setting text for terms that are too internal:
  `ide-check`, `lsp-snapshot`, `seed`, `hybrid`, `native target`, and
  `opaque boundary`.
- Make workflow command examples consistent: prefer `eng.exe run <file.eng>
  --save-artifacts` where saved artifacts are discussed.
- Review stdlib/module status words. Public docs should distinguish
  `supported`, `supported narrow`, `internal`, and `planned` without making
  users parse implementation history.
- Audit workflow helper names for readability, especially
  `select_first_row(...)`, `check coverage`, `predict <model> using <table>`,
  and DB write forms.
- Keep CLI errors and setting descriptions action-oriented: what happened, why
  it matters, and the next valid command.

## Seed-To-Implementation Candidates

- Cache replay/invalidation: current cache records hit/miss and hash policy;
  implement a real replay path only when the artifact contract can prove the
  cached value is safe to reuse.
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

- Add a persistent run history list with timestamp, profile, source file,
  status, and artifact root.
- Make output manifest and artifact rows open files directly from every
  relevant inspector.
- Add panel-level empty states that say which artifact is missing and which
  command produces it.
- Add problem filtering by severity/code and source-line jump targets.
- Add a workflow graph node detail drawer for process/cache/model/DB/case nodes.
- Group raw JSON inspectors behind advanced toggles so first-run users see the
  reviewable summary first.
- Add source span breadcrumbs in case/model/DB/network panels.

## VS Code Linter And Highlighting Candidates

- Move from open/save checks to debounced on-change diagnostics for unsaved
  buffers when no file-relative data reads are needed.
- Promote `eng-lsp` from snapshot mode to a persistent LSP server when the
  protocol surface is stable.
- Add semantic tokens from compiler metadata so symbols, quantities, units,
  functions, and module names can be colored like a mature language extension.
- Add snapshot tests for the TextMate grammar and completion vocabulary.
- Surface quick fixes for common diagnostics such as `:=`, missing units,
  unsupported `script`, and stale `struct Args`.

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
