# Test And CI Gate Map

This page maps the uncertainty, reviewability, and composite-workflow checklist
test names onto the repository's current gate structure. It is intentionally a
current-status document, not a public user guide.

The documentation plans in `EngLang_Documentation_Reorganization_and_User_Guide_Plan.md` and
`EngLang_Documentation_Plan_with_OODocs.md` set two constraints that apply here:

- Public user docs, reference docs, workflow docs, development docs, and internal
  track docs must stay separated.
- OODocs and other documentation publishing helpers are optional publishing layers;
  core runtime and CI evidence must not depend on them.

## Active Gates

| Gate | Command | Current purpose |
|---|---|---|
| Workspace CI | `dev.bat ci` | Fails on Rust formatting drift, then runs workspace tests, example smoke, LSP smoke, JIT checks, clippy, and the package run example. |
| Example workflow smoke | `cargo run -p eng_cli -- test examples` | Checks all official, workflow, advanced solver, internal, and compatibility examples, then runs targeted artifact assertions. |
| Artifact golden check | `dev.bat artifacts-check` | Validates schemas and stable artifact snapshots for official and internal fixtures. |
| Documentation check | `dev.bat docs-check` | Checks Markdown hygiene, links, LLM load-map and code-form document references, retired/stale document wording, snippets, and supported documentation examples. |
| Native workflow status | `dev.bat workflow-native-status` | Quickly reports workflow 01/02/03 native-only source/docs status and latest process/run-graph artifact evidence without rerunning the full smoke gate. |
| LSP/editor check | `dev.bat lsp-check` | Runs library, persistent-stdio, semantic-token, visual-contract, exact-source, token-free-trivia, variable-width scalar-declaration, backward-alias and pure scalar-arithmetic type propagation, mixed fast/explicit style transitions, annotation/RHS/cardinality rechecks, rename/cardinality changes, complete clear/restart, resized/inserted/removed trivia and line-ending suffix rechecks, strict full-fallback, recursive import invalidation, verified imported state-space/class-object/complete component-graph prefix reuse, direct class-object field/method suffix edits, stale semantic-metadata fallback, and system-local name-isolation regressions. |
| VS Code extension check | `dev.bat vscode-test` | Builds LSP editor metadata, emits semantic snapshots for every example and grammar fixture, checks scope mappings and non-overlap, runs grammar and extension contracts, and uses the installed VS Code TextMate runtime to compare role-bearing semantic tokens with first-paint scopes across all examples. |
| Editor visual contract | `dev.bat editor-visual-check` | Builds `eng-lsp`, validates bounded VS Code Light/Dark plus native IDE fixtures, clean/intentional diagnostics, semantic token coverage, inspected screenshot manifest integrity, and the zero-diff comparison engine path. |
| Editor visual comparison | `dev.bat editor-visual-compare <capture-dir>` | Compares supplied Light/Dark/native captures with accepted baselines using fixed dimensions, RGB thresholds, summary JSON, and generated diff PNGs. |
| Package smoke | `dev.bat package-smoke` | Validates the portable public package path. Workflow fixtures stay in workflow smoke until promoted. |

## Editor Visual Acceptance

`tools/editor-acceptance` keeps a user-data-free VS Code workspace, a bounded
native IDE workspace, and one shared clean `main.eng` source. The executable
gate requires zero diagnostics for that source, broad semantic token
type/modifier coverage, and exactly four known diagnostics in the separate
negative fixture. It also checks the headers, dimensions, and SHA-256 values of
the manually inspected VS Code Light, VS Code Dark, and native IDE screenshots,
then exercises their zero-diff pixel-comparison path.

Screenshot replacement remains an explicit manual acceptance step: verify the
EngLang language mode, zero clean-source Problems, role-aware color families,
contrast, clipping, and overlap before updating the manifest. CI protects the
accepted files and compiler/editor contract but does not automate VS Code.
`editor-visual-compare` evaluates supplied same-dimension captures with bounded
RGB thresholds and emits diff artifacts; it does not claim cross-platform pixel
equivalence across OS, font, GPU, or VS Code versions.

The TextMate parity test discovers both unpacked VS Code `node_modules` and
the current `node_modules.asar` plus unpacked Oniguruma WASM layout. When
launched under VS Code Electron, or when runtime coverage is explicitly required,
missing TextMate dependencies fail the gate instead of reporting a skipped pass.
The test compares keyword/modifier tokens and role-bearing function/method tokens;
ordinary identifiers and literals remain outside this cross-engine parity check.
When a semantic token has a mapped modifier, the comparison requires a
modifier-specific fallback scope; a generic token-type scope cannot hide a
role mismatch.

## Checklist 9.1: Uncertainty Tests

Current coverage is split across compiler tests and example artifact smoke:

- Constructor, argument, source, direct-compare, statistic, probability, and
  policy diagnostics are covered by compiler tests in
  `crates/eng_compiler/src/lib.rs` and the diagnostic examples under
  `examples/diagnostics/error_messages`.
- Runtime and artifact evidence is covered by
  `examples/internal/04_uncertainty_core/main.eng` and
  `examples/workflows/03_uncertain_sensor_report/main.eng` in
  `eng test examples`. These checks require review, report, result,
  histogram, and confidence-band artifacts.
- The named fixture paths from the checklist, such as
  `tests/runtime/uncertainty_scalar_linear.eng` and
  `artifacts/uncertainty_review_snapshot`, are represented by these
  current repo gates rather than by matching file names.

Remaining gap: broad stable Monte Carlo/Jacobian propagation and full
probabilistic TimeSeries semantics remain internal. Do not promote uncertainty
as public-supported from these gates alone.

## Checklist 9.2: Reviewability Tests

Current coverage:

- Review contract, fallback, side-effect, and risk sections are covered by
  compiler tests around `review_json` in
  `crates/eng_compiler/src/lib.rs`.
- Where/with expansion and side-effect artifact assertions are covered by
  official example smoke paths, especially
  `examples/official/09_command_where_with/main.eng` through
  `examples/official/16_test_assert_golden/main.eng`.
- Shared ReviewDocument extraction, validation, and semantic diff unit tests
  live in `crates/eng_compiler/src/review_diff.rs`; CLI parser and
  call-site coverage remains in `crates/eng_cli/src/main.rs`.
- Compiler tests also prove deterministic runtime section rehashing, unchanged
  static-hash preservation, and `symbols`/`config_promotions` hash coverage.
- Runtime tests project declared-unit scalar results, schema/table evidence,
  materialized tables, TimeSeries, explicit coverage checks, source-derived
  time axes, table-transform row counts, and validation outcomes into matching
  normalized rows, then compare the static and runtime documents through the
  shared diff engine. The coverage regression checks the same table and
  coverage result across units/quantity, symbol, derived-value, and calculation
  rows.
- Report tests accept full and bare validated ReviewDocuments, reject incomplete
  documents, prefer normalized validations over legacy ReportSpec validation
  rows, and guard unit-safe runtime summaries. Runtime, artifact, and workflow
  gates assert that `report.html` contains the same semantic fingerprint as
  the final saved `review.json` plus the Runtime Review result/evidence
  columns. Workflow 03 additionally requires both normalized time-axis rows,
  native table/TimeSeries/coverage provenance, exact sample counts, and the
  matching HTML coverage summary.
- The example smoke runs `eng review --output`, `eng review --against`,
  and `eng review diff <old> <new>` through the built CLI binary. It asserts
  that `static_review.json` and `semantic_diff.json` are written and
  that the direct and `--against` payloads match exactly.
- IDE smoke checks native inspector payloads for runtime table transform rows
  when the `tests/runtime/table_datetime_comparison.eng` fixture is available
  in the repo workspace.
- IDE bootstrap exposes `stdlib/eng/modules.toml` entries to the Modules panel
  with status, backing, purpose, and artifact metadata.
- IDE command tests verify that wrapped and bare ReviewDocuments reach the
  shared engine and reject incomplete input. `ide-check` also guards the
  baseline picker, automatic refresh path, section/item renderer, and bounded
  Tauri command wiring. The same gate guards runtime value/status rendering in
  the native IDE and source-hash-matched last-run projection in the VS Code
  Review panel.

## Checklist 9.3: Composite Module Tests

Current coverage uses generic artifact contracts rather than native module
syntax:

- `runtime/table_filter_station_map.eng`, `runtime/table_select_columns_station_map.eng`,
  `runtime/table_derive_station_map.eng`, `runtime/table_sort_station_map.eng`,
  `runtime/table_require_one.eng`,
  `runtime/table_datetime_comparison.eng`,
  `runtime/table_row_diagnostics_station_map.eng`,
  `runtime/table_join_samples_results.eng`, and
  `runtime/timeseries_coverage.eng` map to the weather workflow smoke.
  They check deterministic station selection, filter/select/derive/sort/require_one/join table
  transform artifacts, Date/DateTime predicate comparison, row-level
  diagnostics, and Gregorian-year coverage artifacts without real
  network access.
- `runtime/config_optional_fields.eng` covers typed JSON config promotion with
  optional missing/null fields recorded in result and review artifacts.
- `runtime/case_manifest.eng` and
  `runtime/process_expected_outputs.eng` map to the external
  simulation workflow smoke. It checks case input/result/manifest outputs,
  process expected-output status, tool versions, stdout/stderr hashes, and
  enriched case manifests.
- `runtime/model_card_external.eng` maps to surrogate trainer
  artifacts: `outputs/surrogate.json`, `outputs/model_metrics.json`,
  and `outputs/model_card.json`; runtime review payloads adapt model cards into
  `typed_payload.model_specs[]`, `typed_payload.model_cards[]`, and
  `typed_payload.model_diagnostics[]`.
- `examples/workflows/02_native_surrogate_case_workflow/main.eng`
  also covers prediction-manifest adaptation into
  `typed_payload.prediction_manifests[]`, including output quantity/unit,
  case IDs, row counts, hashes, and confidence-column metadata. Its saved
  ReviewDocument gate also requires four discriminated model/prediction symbol
  results, three model evidence records, one prediction evidence record,
  computed metrics and coefficients, model/training/prediction hashes, and the
  matching Runtime Review HTML rows.
- Native DB tests cover `open sqlite`, `write <table> to db.table("...")`,
  append, upsert with key, replace, typed SQLite table readback, transaction
  rollback, schema mismatch diagnostics, safe-profile rejection, DB write
  manifests, SQLite database artifacts, DB file hashes before/after, and table
  records.
- Native model predict tests cover `predict <model> using <table>`,
  Table[Prediction] materialization, prediction-manifest metadata, confidence
  columns, and writing prediction tables through the native SQLite DB write
  path.
- Native case-runner tests cover sequential and bounded parallel scheduler
  policy validation, actual concurrent row evaluation, deterministic uneven
  partitions and result order, dependent result expressions, lifecycle hooks,
  worker telemetry, result/run manifests, and scheduler-aware cache identity.

Remaining gap: broader `eng.cache` invalidation/reuse policy, broader sampling
 distributions/design policies, DB query/engine support, and public model
 training syntax remain planned. Pinned response/cache-record
evidence must stay labeled as composite workflow foundations, not full native
module support.

## Checklist 9.4: Workflow Example Tests

Current coverage:

- `examples/workflows/01_weather_api_to_standard_file/main.eng`
  runs in deterministic pinned-response mode without a real network call;
  runtime tests separately cover live HTTP(S) execution and cached replay.
- `examples/workflows/02_native_surrogate_case_workflow/main.eng`
  runs native sampling, template rendering, regression, prediction, and DB writes
  with zero external process executions.
- `examples/workflows/03_uncertain_sensor_report/main.eng`
  runs typed sensor data and report generation with uncertainty metadata and
  zero external process executions.
- `eng test examples` asserts that all three workflows produce review,
  output-manifest, process, report, and typed result artifacts.
- `eng test examples` and `dev.bat workflows-test` reject external-process adapters,
  external scripting/library markers, and legacy row-selection helpers across every `.eng` source
  under the three native workflow directories and rejects process/run-command/external-scripting nodes in saved run graphs; `workflows-test` also rejects stale public-doc wording,
  malformed process-results artifacts, non-normal workflow smoke profiles, and nonzero process counts.
- The same gate checks workflow 01 pinned-response cache materialization/replay
  evidence, workflow 02 structured sampler/model/prediction/DB/case evidence
  including unchanged typed values through materialize/apply/collect and
  downstream model/filter/DB consumption of the final collection,
  and workflow 03 propagated uncertainty, report, and confidence-band plot
  evidence so those workflows cannot pass by reading file-backed surrogate data.
- `dev.bat ide-check` also executes workflow 03 and rejects an uncertainty
  inspector that omits sensor declarations, actual runtime calculations, or
  explicitly non-executed static plans, or that restores the old ambiguous
  calculation key.
- The native IDE editor-safety and structural gates also require metric,
  validation, and alignment results to remain in Checks while solver results,
  state-space operators, and equation dependencies render in the separate
  System panel with semantic-token navigation.
- `dev.bat workflow-native-status` provides the fast read-only status view for those same source/docs guards plus the latest process and run-graph artifacts.
- These examples are intentionally covered by workflow smoke rather than public
  package smoke until their native modules are promoted into package scope.
