- Batch 933: Replaced the misleading `fill_missing` editor completion with canonical `check coverage` and `fill missing` workflow snippets, so completion inserts public EngLang syntax and its explicit policy blocks instead of an internal strategy spelling.
- Batch 932: Removed the compatibility-only CaseOutput planned_count member alias from compiler/runtime member lookup and guarded the public API on expected_count.
- Batch 931: Split workflow docs between the implemented native materialize/apply/collect case-table path and the remaining general run-case scheduler work, and renamed internal case-manifest seed test labels to record labels.
- Batch 930: Made VS Code Tooling Status native workflow summaries explicitly say no external processes alongside process_count=0 so zero-process evidence is readable without interpreting raw counters.
- Batch 929: Renamed workflow case-table artifact runner status from manifest_seed_runner to native_template_runner and added smoke/runtime guards so native case execution no longer reads as seed-only.
- Batch 928: Added native primitive evidence summaries to workflow-native-status and VS Code Tooling Status so workflow 01/02/03 show concrete native API, sampling/model/DB, and uncertainty/report primitives instead of only no-Python checks.
- Batch 927: Mirrored domain helper TextMate scopes inside string interpolation so model, uncertainty, TimeSeries, and solver helper calls color before the generic builtin fallback.
- Batch 926: Colored parent(...), stem(...), and extension(...) as path helper TextMate calls alongside join(...), including interpolation coverage, grammar smoke, and token-scope docs.
- Batch 925: Colored exists(...) as an external-boundary TextMate helper instead of the last generic builtin first-paint helper, with grammar smoke and token-scope docs updated.
- Batch 924: Reworded VS Code README quick-fix merge and cache invalidation text away from fallback-repair/state wording toward local quick fixes and cached review/highlight data, with a contract guard.
- Batch 923: Reworded VS Code README model source-chain quick-fix coverage from skeleton repairs to starter-code repairs and guarded against skeleton-repair wording returning.
- Batch 922: Added theme fallback scope summary aliases to VS Code highlight inspector and Tooling Status probe payloads, keeping legacy fallback_scope_status fields for compatibility.
- Batch 921: Reworded internal_planned stdlib module labels/details from Internal planned and target-surface wording to Internal target and explicit public stdlib API boundaries, then regenerated VS Code editor metadata.
- Batch 920: Added theme_coverage_status and theme_fallback_scope_count to VS Code highlight inspector/copy payloads while keeping fallback_status compatibility fields, with README and contract coverage.
- Batch 919: Reworded VS Code planned/internal stdlib import hover text away from workflow-surface/internal-boundary wording toward explicit public stdlib API status, with an ide-check guard.
- Batch 918: Reworded VS Code highlight-inspector README coverage wording from mapped/missing fallback status to theme fallback coverage state, with an ide-check guard.
- Batch 917: Reworded current docs away from raw JSON/source-diff phrasing toward manual JSON artifact and line-by-line source comparison wording, with docs-check guards.
- Batch 916: Reworded the native IDE how-to Tooling Status summary from the old fallback-status phrase to configured-path/source status and guarded user docs against the old phrase.
- Batch 915: Reworded VS Code Tooling Status configured executable path states away from fallback wording toward explicit discovered-tool labels and guarded against the old fallback phrasing returning.
- Batch 914: Renamed VS Code highlight inspector advanced payload fields from `raw` to `advanced_highlight_data` and guarded against the raw payload label returning.
- Batch 913: Added local VSIX package/install freshness and install preflight guidance to VS Code Tooling Status when the active workspace is an EngLang source checkout.
- Batch 912: Split the remaining workflow-step, solver, and path helper TextMate scopes so `apply(...)`, `run_case`, `der(...)`, and `join(...)` no longer use generic builtin first-pass colors.
- Batch 911: Added an external-boundary helper TextMate scope so `file(...)`, `url(...)`, `env(...)`, and `secret env(...)` use external-boundary first-pass colors instead of generic builtin colors.
- Batch 910: Added a TimeSeries/statistic helper TextMate scope so `integrate(...)`, `mean(...)`, `time_weighted_mean`, and `p90` use TimeSeries-specific first-pass colors instead of generic builtin colors.
- Batch 909: Added an uncertainty helper function TextMate scope so `measured(...)`, `uniform(...)`, `propagate(...)`, and `probability(...)` use uncertainty-specific first-pass colors instead of generic builtin colors.
- Batch 908: Added a model helper function TextMate scope so `train_test_split(...)`, `regression(...)`, `evaluate(...)`, and `model_card(...)` use model-specific first-pass colors instead of generic builtin colors.
- Batch 907: Colored model workflow phrase keywords such as `predict ... using ...` and `train regression ...` with the model TextMate scope so first-pass VS Code colors match LSP model roles sooner.
- Batch 906: Added TextMate first-paint coverage for class object construction and copy-with headers so object source names and `with` use model-colored scopes before semantic tokens arrive.
# Usability Improvement Backlog

- Batch 905: Added TextMate first-paint coverage for `rmse measured.T vs simulated.T` comparison phrases so the validation metric, `vs` operator, and dotted measured/simulated operands are colored consistently before semantic tokens arrive.
- Batch 904: Extended native workflow guards so workflow 01/02/03 audit every support-file path plus README/text docs for Python, `.py`, notebook, and run-command regressions instead of only checking executable `.eng` sources.
- Batch 903: Added an LSP regression guard that scans every `examples/**/*.eng` source and fails if any keyword semantic token falls back to an empty modifier set, keeping role-aware keyword coloring from regressing.
- Batch 902: Marked the domain conservation `is shared at connected ports` connector as solver-colored and verified all example keyword semantic tokens now carry a role modifier instead of falling back to generic keyword colors.
- Batch 901: Marked class/object copy-with headers so `source with {` keeps the source object and `with` in the model semantic color family, including invalid in-progress copy expressions before semantic object info is available.
- Batch 900: Colored measured-vs-simulated RMSE comparison expressions so `rmse measured.T vs simulated.T` marks `vs` and both dotted operands with report/timeseries/validation semantic modifiers instead of generic keyword/property colors.
- Batch 899: Marked statement-leading `return` keywords with the local semantic modifier so function return lines use role-aware editor coloring without repainting catalog literals named `return`.
- Batch 898: Marked validation/filter condition words such as `is`, `none`, `or`, and range `and` with validation semantic modifiers so schema constraints and `where` filters no longer leave boolean condition syntax as generic keyword colors.
- Batch 897: Marked `use` and `import` keywords with declaration/imported semantic modifiers so file and namespace imports use the same editor color family as imported namespace tokens.
- Batch 896: Marked contextual `linear` constant literals with uncertainty/model semantic modifiers so propagation methods, regression algorithms, and uncertainty options do not fall back to generic keyword colors.
- Batch 895: Marked equation `eq` and TimeSeries `of` operator words with solver/timeseries semantic modifiers so checked-code highlighting no longer leaves equation and series type clauses as generic keywords.
- Batch 894: Marked declaration keywords such as `schema`, `domain`, `component`, `system`, `class`, `fn`, `method`, `package`, and `version` with declaration semantic modifiers so checked-code highlighting preserves declaration colors instead of generic keyword colors.
- Batch 893: Added role-aware fallback coloring for partial workflow/report clause keywords so `sort ... by`, `join ... with`, `summarize ... by`, repeated plot `and`, `show ... as`, and summary field `as/with` no longer fall back to generic keyword colors.
- Batch 892: Colored `connect ... to ...` solver endpoints and `to` clauses in LSP fallback highlighting so component connection lines no longer drop to generic keyword/property colors before richer semantic context.
- Batch 891: Made partial DB table read/write targets role-aware in LSP fallback highlighting so `write ... to args.database.table(...)`, `read sqlite ... as ...`, and copy/move `to` clauses keep DB/external colors before full type resolution.
- Batch 890: Marked export-summary field connectors as report-aware semantic tokens so `as` and `with` in summary CSV field rows use report role colors instead of generic keyword coloring.
- Batch 889: Clarified workflow 02 case status wording by separating initial CaseTable manifest state from final CaseResultCollection status in bindings, prints, and docs.
- Batch 888: Made VS Code saved-file uncertainty argument diagnostics underline invalid named argument values such as `samples=many` instead of the uncertainty constructor name.
- Batch 887: Routed VS Code saved-file `E-WITH-UNIT-001` diagnostics through option-value fallback ranges so incompatible report display units underline the requested unit value.
- Batch 886: Made VS Code saved-file Problems fallback ranges parse print/write interpolation fields so format unit diagnostics underline the requested unit instead of the expression binding.
- Batch 885: Added workflow-aware semantic modifiers to network, process, report, and solver with blocks so their options and helper values stay visible in workflow highlight filters.
- Batch 884: Made VS Code and native IDE highlight coverage phrase-aware so split role-aware tokens still cover multi-word workflow options such as `unit x` and `unit y`.
- Batch 883: Marked model workflow `with` blocks as workflow-aware semantic keyword ranges so the block keyword matches the model options it contains.
- Batch 882: Fixed CSV promotion member completion so schema columns remain reachable when generic table fields do not match the typed member prefix.
- Batch 881: Promoted model workflow connectors (`from`, `on`, `using`) and model with-block options to workflow-aware highlighting in TextMate and LSP semantic tokens.
- Batch 880: Set the VS Code and native IDE Public fields highlight coverage filter to `property`, matching the semantic token selectors users actually filter in highlight tables.
- Batch 879: Aligned native IDE Highlight coverage inspection with VS Code by adding Types, Quantities, and Public fields from the shared editor metadata catalog.
- Batch 878: Expanded VS Code highlight coverage inspection to report Types, Quantities, and Public fields from shared editor metadata, so color gaps outside keyword/unit scopes are easier to see.
- Batch 877: Made `workflow-native-status` explicitly report no Python/.py/notebook/run-command markers, no external processes, and no process/Python run-graph nodes for workflow 01/02/03 evidence.
- Batch 876: Colored `write text`, `write json`, and `write standard_text` format selectors as side-effect first-paint tokens and added grammar guard coverage for those phrase captures.
- Batch 875: Added `standard_text` to shared editor workflow keyword metadata so VS Code and native IDE keyword catalogs expose the native standard-file writer consistently.
- Batch 874: Split VS Code first-paint theme colors for function declarations, calls, built-in helpers, member fields, and public members so code is more readable before role-aware semantic colors arrive.
- Batch 873: Split base VS Code EngLang theme colors for functions, methods, and properties so generic calls and member fields remain visually distinct before domain-specific role colors apply.
- Batch 872: Removed the remaining raw internal metadata wording from the native IDE how-to and guarded against that phrase returning in user-facing IDE docs.
- Batch 871: Reworded LSP/VS Code read text/json/toml completion details from raw read wording to direct read wording, regenerated editor metadata, and added guards against the older completion details returning.
- Batch 870: Reworded the native IDE highlight detail expander from raw JSON wording to Advanced highlight data, renamed the shared expander class/helper accordingly, and added IDE guards against the older raw label returning.
- Batch 869: Reworded the portable user guide away from raw artifact-file wording toward explicit JSON artifact files, with a user-docs guard against the older phrase returning.
- Batch 868: Reworded workflow 01 API docs and printed output away from generic payload wording toward native HTTP response and API JSON contract language, with docs guards against the older phrases returning.
- Batch 867: Archived current backlog batches 841-856 into the historical log so active API/IDE/workflow cleanup docs stay focused on the latest linter, highlighting, and native IDE usability work.
- Batch 866: Added native IDE Copy at cursor for Problems plus source-line diagnostic copy text so linter reports can be captured from the caret without reconstructing context manually.
- Batch 865: Reworded VS Code README copy/inspect guidance from implementation-detail copy wording to copy-ready diagnostic/highlight details and advanced highlight data.
- Batch 864: Added role-aware highlighting aliases to VS Code highlight inspector payloads and reworded remaining highlight warnings/status summaries away from semantic-highlighting implementation terms.
- Batch 863: Added native IDE status-label mappings for behavior graph not-connected solver states so advanced solver panels avoid leaking raw `*_not_integrated` artifact keys.
- Batch 862: Reworded VS Code Tooling Status and current docs away from implementation-detail highlighting internals toward checked-code role-aware color wording, with a package guard against the older phrasing returning.
- Batch 861: Moved the native IDE Highlight tab into the primary review flow immediately after Review, with side-tab order guards and user-doc wording so color/range debugging is easier to find.
- Batch 860: Color-coded native IDE Highlight panel chips for token categories, details, selectors, and coverage domains so the inspector mirrors the editor's role colors instead of showing mostly generic chips.
- Batch 859: Reworded VS Code and native IDE user-facing highlighting settings/docs from implementation-detail phrasing to checked-code and role-aware wording, with guards against the older wording returning.
- Batch 858: Split VS Code EngLang theme colors across unit, quantity, TimeSeries, workflow, validation, report, side-effect, external, solver, model, DB, and cache role families so role-aware highlighting is more colorful without collapsing each family into one color.
- Batch 857: Reworded VS Code highlighting docs away from TextMate/semantic implementation phrasing toward first-pass syntax colors and role-aware colors, with a guard against the older wording returning.

This file is the short current backlog for API clarity, native workflow usability,
editor/linter behavior, and documentation cleanup. The historical batch log was
archived to [usability_improvement_backlog_history.md](../archive/usability_improvement_backlog_history.md).

## Current Priorities

1. Keep workflow 01/02/03 native-only: no Python, `.py`, or `run command` path may re-enter those examples.
2. Replace remaining seed-only implementation paths with executable compiler/runtime behavior where the public docs imply support.
3. Improve VS Code and native IDE authoring quality: consistent TextMate first-paint highlighting, checked-code role-aware colors, precise diagnostics, hover, completion, and quick fixes.
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
- VS Code linter/highlighting: continue expanding checked-code role-aware color coverage as more source spans become first-class metadata.

## Documentation Policy

- Public behavior changes update user docs, reference docs, examples, and release notes when applicable.
- Runtime artifact changes update the artifact reference and schemas when their shape changes.
- Historical implementation logs belong in `docs/archive`; internal unstable design work belongs in `docs/internal` or focused `docs/current/*_plan.md` files.
- `docs/README.md` remains the navigation entry point; avoid adding parallel indexes unless they serve a specific reader path.
