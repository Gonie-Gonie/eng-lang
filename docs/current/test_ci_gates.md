# Test And CI Gate Map

This page maps the uncertainty, reviewability, and composite-workflow checklist
test names onto the repository's current gate structure. It is intentionally a
current-status document, not a public user guide.

The documentation plans in `EngLang_Documentation_Reorganization_and_User_Guide_Plan.md` and
`EngLang_Documentation_Plan_with_OODocs.md` set two constraints that apply here:

- Public user docs, reference docs, workflow docs, development docs, and internal
  track docs must stay separated.
- OODocs or other Python documentation tooling is an optional publishing layer;
  core runtime and CI evidence must not depend on it.

## Active Gates

| Gate | Command | Current purpose |
|---|---|---|
| Workspace CI | `dev.bat ci` | Runs Rust formatting, workspace tests, example smoke, LSP smoke, JIT checks, clippy, and the package run example. |
| Example workflow smoke | `cargo run -p eng_cli -- test examples` | Checks all official, workflow, advanced solver, internal, and compatibility examples, then runs targeted artifact assertions. |
| Artifact golden check | `dev.bat artifacts-check` | Validates schemas and stable artifact snapshots for official and internal fixtures. |
| Documentation check | `dev.bat docs-check` | Checks documented snippets and supported documentation examples. |
| Package smoke | `dev.bat package-smoke` | Validates the portable public package path. Workflow fixtures stay in workflow smoke until promoted. |

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
- `eng review` semantic diff unit tests live in
  `crates/eng_cli/src/main.rs`.
- The example smoke now also runs `eng review --output` and
  `eng review --against` through the built CLI binary and asserts that
  `static_review.json` and `semantic_diff.json` are written.
- IDE smoke checks native inspector payloads for runtime table transform rows
  when the `tests/runtime/table_datetime_comparison.eng` fixture is available
  in the repo workspace.
- IDE bootstrap exposes `stdlib/eng/modules.toml` entries to the Modules panel
  with status, backing, purpose, and artifact metadata.

Remaining gap: the separate planned spelling `eng review diff <old> <new>`
and a native IDE semantic diff panel are still planned.

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
  case IDs, row counts, hashes, and confidence-column metadata.
- Native DB tests cover `open sqlite`, `write <table> to db.table("...")`,
  append, upsert with key, replace, typed SQLite table readback, transaction
  rollback, schema mismatch diagnostics, safe-profile rejection, DB write
  manifests, SQLite database artifacts, DB file hashes before/after, and table
  records.
- Native model predict tests cover `predict <model> using <table>`,
  Table[Prediction] materialization, prediction-manifest metadata, confidence
  columns, and writing prediction tables through the native SQLite DB write
  path.

Remaining gap: broader `eng.cache` invalidation/reuse policy, broader sampling
distributions/design policies, case-runner scheduling, DB query/engine support,
and public model training syntax remain planned. Pinned response/cache-record
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
- `dev.bat workflows-test` rejects `run command`, Python calls/library markers, malformed
  process-results artifacts, non-normal workflow smoke profiles, and nonzero
  process counts in the three native workflow examples.
- The same gate checks workflow 02 structured sampler, model-card, prediction, DB,
  and case-manifest evidence so it cannot pass by reading file-backed surrogate data.
- These examples are intentionally covered by workflow smoke rather than public
  package smoke until their native modules are promoted into package scope.
