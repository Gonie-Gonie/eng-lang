# Current Project Status

This page is the authoritative short-form status layer for contributors and LLM
agents. It separates public release versions from long-term development tracks.

## Release State

| Field | Value |
|---|---|
| Current public line | `v0.9-preview` |
| Active target | `v1.0` stable core hardening |
| Workspace package version | `0.9.0-preview` |
| Release channel | `preview` |

EngLang is preview software. The language, runtime behavior, and artifact
formats are not stable. Earlier high-numbered release names are not part of the
current public version line.

The active language philosophy is recorded in
[Integrated Language Philosophy](philosophy.md):

```text
EngLang is a unit-safe engineering programming language for typed data
analysis, system simulation workflows, plotting, and reproducible review.
```

Future capabilities are tracked in [development tracks](tracks.md), not as
public release versions. A track may have implementation seeds on `main` without
being part of the public release contract.

## Core Execution Invariants

- Core checking, running, plotting, report generation, and packaged execution
  do not depend on Python.
- Python may be used for optional documentation tooling only.
- The official execution path is `.eng -> typed semantic model -> bytecode ->
  native runtime/VM -> result/report/PlotSpec objects`; `--save-artifacts`
  writes `.engbc`, `.engres`, SVG/HTML/report/review artifacts and
  `run_log.json`, `process_results.json`, `test_results.json`, and
  `output_manifest.json`.
- Fast declaration uses `=`. `:=` is rejected.
- Physical equations use `eq`. `==` is comparison syntax and is rejected in
  equation blocks.
- Public features need examples, tests, diagnostics or metadata where relevant,
  and reviewable artifacts.

## Supported Preview Features

Supported preview means the behavior is documented, tested, and usable through
the current public preview workflow, but it is not yet a stable contract.

- Fast `=` declarations in script/local expression contexts and top-level
  executable workflows.
- Unit and quantity checking for supported arithmetic and official examples.
- Dimensionless plus physical quantity diagnostics.
- Ambiguous quantity warnings for unit-only declarations such as `power = 10 kW`.
- Top-level execution as the default root workflow, root `args { ... }`
  blocks, importable top-level `const`, pure scalar `fn` definitions with
  typed parameters and function-local bindings, checked return dimensions,
  relative file imports for importable declarations, and no imported
  executable-body side effects.
- Parenthesis-light command syntax for built-in workflow verbs only, with
  canonical lowering metadata, `where` owner-local calculation blocks,
  `with` option/display blocks, and policy diagnostics for ambiguous command
  targets, where-local scope, forward references, unknown options, and
  incompatible display units.
- Typed CSV promote for the official typed schema import path.
- DateTime-indexed table metadata and row-level CSV runtime pages.
- TimeSeries statistics on the official HeatRate path, including mean,
  time-weighted mean, median, standard deviation, percentiles, and trapezoidal
  integration metadata, with `timeseries_kernels` metadata for the preview
  table heat-rate expression.
- PlotSpec v1 line plot data, unit-aware axis labels, SVG export, and plot
  manifest artifacts.
- Report/review artifacts with variable tables, inferred declarations, unit
  conversion records, schema summaries, warnings, plot manifest data, and
  report spec hashing.
- Minimal `system`/`eq` parsing and unit diagnostics, with one-state thermal
  system metadata and fixed-step preview execution for official examples.
- Args string/path/CsvFile/DirectoryPath binding for `--input` style official
  examples, primitive Bool/Int/Count/Float/Duration normalization, dynamic
  pure defaults, and packaged runner help metadata.
- Typed path helper seed: `file`, `dir`, `join`, `parent`, `stem`,
  `extension`, and `exists` work for path-oriented workflow values. `exists`
  records review/result/report-spec `environment_dependencies` provenance.
- Read-only UTF-8 `read text`, `read json`, and `read toml` expression forms
  resolve source-relative paths at runtime, return raw text strings, and record
  source path plus source hash provenance in review/result/report-spec
  `environment_dependencies`.
- Explicit `write text` and `write json` top-level workflow statements write
  under `build/result`, require `with { overwrite = true }` when replacing a
  file with different contents, and are recorded in `review.json`.
- `export summary to csv` uses the same idempotent overwrite hardening: an
  identical existing file is accepted, while different contents require an
  attached `with { overwrite = true }` block.
- `output_manifest.json` records generated file artifacts and content hashes
  for saved runtime artifacts, CSV exports, and write outputs.
- Explicit `copy`, `move`, and `delete` top-level workflow statements provide
  a constrained filesystem mutation seed. Copy can bring a source-relative
  text file into `build/result`; move/delete operate on generated output paths;
  move/delete require `with { confirm = true }`; directory delete also requires
  `recursive = true`; all operation records appear in review/output manifest
  metadata.
- `print` remains lightweight CLI/debug output. `log debug/info/warn/error`
  creates structured runtime message metadata. Saved runs write
  `run_log.json`, and `output_manifest.json` records the run-log artifact.
- `run command "..."` statements bind a `ProcessResult`, execute explicitly
  declared external processes, support `with { args = [...], cwd = ..., allow_failure = true }`,
  and write saved `process_results.json` artifacts.
- Named `test` blocks group checked `assert` statements and `golden` artifact
  comparisons. Saved runs write `test_results.json`, and failed tests fail the
  run after artifacts are available for inspection.
- Standalone package output with `.engpkg`, bytecode, lock, source/dependency
  copy, dependency hashes, Args help, and reviewable report artifacts.
- Temperature spelling policy: `degC` remains the canonical ASCII spelling, and
  `°C` is supported as a user-facing alias for `AbsoluteTemperature`.
- Example taxonomy: `examples/official` is the user-test namespace; top-level
  numbered examples are compatibility regression paths; diagnostic and
  data-quality fixtures are separated by folder.

## Preview Tooling

- Native tester IDE (`eng-ide.exe`) for open/check/save/run, diagnostics,
  completions, source editing, variable/unit/schema/CSV inspection, PlotSpec
  preview, runtime summaries, UI settings, and artifact links.
- VS Code extension preview packaged as a secondary editor path.
- Integrated HVAC user-test example combining CSV promote, statistics, plotting,
  reporting, and system metadata.
- Data-quality examples for parse failures, missing-value interpolation,
  constraint violations, and unit conversion failures.
- TimeSeries raw-value histogram PlotSpec path through `plot histogram(...)`,
  with bin metadata shared with future-track distribution plots.
- Unit-aware `print` interpolation and explicit one-row
  `export summary to csv` output under `build/result`; this is not a
  first-class Summary object model.
- Structured runtime message levels through `log debug`, `log info`,
  `log warn`, and `log error`, with `run_log.json` for IDE/tool inspection.
- External process execution through `result = run command "tool"`, with
  review metadata and `process_results.json` for exit/stdout/stderr records.
- Test/assert/golden workflow checks through named `test` blocks, with
  `test_results.json` for IDE/CI inspection.
- OODocs grammar PDF generation through `dev.bat grammar-docs`, backed by the
  language grammar guide.
- Current planning and release docs now align around the integrated
  data-analysis plus system-simulation philosophy and the
  [side-effect policy](../reference/side_effect_policy.md). The implemented
  side-effect scope is GP-1 path helpers, GP-2 read-only UTF-8 text/json/toml
  reads with source hashes, GP-3 write/export hardening with output manifest,
  GP-4 constrained copy/move/delete file operation metadata, GP-5 structured
  log-level runtime messages with run-log artifacts, GP-6 explicit external
  process execution with `ProcessResult` artifacts, and GP-7 test/assert/golden
  verification artifacts.

## Future Tracks On Main

The following tracks may have implementation seeds, examples, tests, and IDE
metadata on `main`, but they are not public release versions:

- Uncertainty track
- Data-driven modeling track
- IDE/LSP track
- Runtime optimization/JIT/AOT track
- Domain/component track
- Class/domain-object track
- General programming and side-effect track

See [development tracks](tracks.md) for the current scope and limitations.

## Deferred / Known Limitations

- Arbitrary table formulas are not fully general.
- Arbitrary TimeSeries expressions are limited beyond the official typed CSV
  path.
- General quantity rules for all statistics are not complete.
- Plot semantics beyond current PlotSpec paths need multi-series, custom
  histogram bin counts, and grouped/stacked bar hardening.
- Multi-state, nonlinear, adaptive, or general equation-system solving is
  deferred.
- Numeric component graph solving and domain package registries are deferred.
- Quantity/unit-literal Args conversion beyond primitive typed Args is
  deferred.
- Full Unicode unit spelling support beyond the supported `°C` alias is
  deferred.
- Package/module imports, multi-return functions, broad function-body statement
  execution, full formatter policy, and stricter reproducibility profiles for
  runtime-dependent defaults are deferred.
- Parenthesis-light syntax for arbitrary user-defined/general function calls
  and project-wide display unit policy blocks are deferred.
- Broad filesystem/network side-effect runtime support is deferred to the
  general programming track. `v0.9-preview` implements path helpers, `exists`
  provenance, read-only UTF-8 text/json/toml source hash provenance, explicit
  write/export output manifest support, constrained output-area
  copy/move/delete, structured runtime log artifacts, explicit external process
  execution, and local test/assert/golden checks only.
- Class/domain objects are planned for reviewable engineering objects, but
  class declaration/object literal/runtime lowering is not part of the current
  public preview.

- First-class Summary objects are not part of the current scope; the v0.2
  decision is recorded in
  [summary_object_decision.md](../reference/summary_object_decision.md).

## Current Crate Architecture

The supported current workspace structure is intentionally compact:

| Crate | Role |
|---|---|
| `eng_cli` | CLI commands, package/release smoke paths, user-facing execution |
| `eng_compiler` | Lexer, parser, AST, semantic checks, units, quantities, bytecode metadata |
| `eng_jit` | Experimental hot-kernel detection and numeric lowering-plan metadata |
| `eng_runtime` | Runtime execution, VM seed, CSV/data policies, `.engres` output |
| `eng_report` | PlotSpec/SVG/report/review rendering and artifact schemas |
| `eng_ide` | Native tester IDE and package smoke UI checks |
| `eng_lsp` | Experimental editor-service smoke and snapshot paths |

Future crate splitting should be documented as planned work, not assumed as the
current architecture.
