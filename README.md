# EngLang

EngLang is a native programming language project for typed engineering data
analysis, system simulation workflows, plotting, and reproducible review. Its
goal is to let the compiler and runtime understand units, physical quantity
kinds, schemas, axes, statistics, plotting, reports, and provenance as
first-class parts of engineering code.

## Status

Current public line: `v1.0.0`

Active target: `v1.0.x` - stable core maintenance and scoped additions

EngLang `1.0.0` is a stable-core release. The documented data-to-report
workflow, artifact family, packaged runner, and native tester path are the
stable contract. Internal implementation seeds remain outside that contract.
EngLang 1.0.0 is not a complete engineering simulation solver.

Start from these short status documents:

- [Current project status](docs/current/status.md)
- [Integrated language philosophy](docs/current/philosophy.md)
- [Version plan](docs/current/version_plan.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [Stable core scope](docs/current/stable_core_scope.md)
- [Development tracks](docs/current/tracks.md)
- [Implementation issue backlog](docs/current/implementation_issue_backlog.md)
- [Breaking change policy](docs/reference/breaking_change_policy.md)
- [LLM context](LLM_CONTEXT.md)
- [LLM load map](docs/llm/load_map.yml)

## Stable Core Workflows

- Typed CSV promote through official examples
- Unit-aware TimeSeries calculations
- TimeSeries statistics and integration metadata
- Measured-vs-simulated workflow with RMSE, validation, time alignment, and
  multi-series PlotSpec
- Unit-aware print and explicit summary CSV export
- Typed path helpers and provenance-visible `exists`
- Read-only UTF-8 `read text/json/toml` with source hash provenance
- Explicit `write text/json`, CSV overwrite hardening, and output manifest
- Explicit `copy/move/delete` file operation seed with confirmation metadata
- `print` plus `log debug/info/warn/error` runtime messages with `run_log.json`
- Explicit `run command` process execution with `ProcessResult` and
  `process_results.json`
- Named `test` blocks with checked assertions, golden artifact comparisons, and
  `test_results.json`
- `eng run --profile safe|normal|repro` runtime policy basics
- PlotSpec/SVG output
- Review/report artifacts
- Basic packaged execution
- Native tester IDE for user testing

Planned and internal work is managed by tracks, not by high-numbered public
versions. These links are roadmap/context entry points, not stable workflow
claims:

- Core language
- Data boundary
- Statistics, plot, and report
- System/equation
- IDE/LSP
- Uncertainty
- Data-driven modeling
- Runtime optimization/JIT/AOT
- Domain/component
- Class/domain-object
- General programming/side-effect policy

The system/equation path currently means one-state fixed-step thermal workflow
support, plus solver metadata and solver-plan artifacts for that narrow scope.
General solvers, DAE, adaptive or multi-state solving, and numeric
component-graph solving remain future tracks. Domain package registries also
remain future work. Historical tags may remain for traceability; current public
release line starts from v1.0.0.

## Quick Start

On Windows, use the root `dev.bat` wrapper for development commands. It
bypasses PowerShell execution-policy issues and keeps the toolchain local to
the repository.

```bat
.\dev.bat setup
.\dev.bat doctor
.\dev.bat ci
.\dev.bat docs-check
.\dev.bat ide --smoke
.\dev.bat artifacts-check
.\dev.bat run-example
```

`setup` installs the pinned Rust toolchain, repo-local MinGW GNU build support,
and a portable Python documentation toolchain into `.dev`, fetches
dependencies, and builds the workspace. A global Rust, Python, MinGW, or Visual
Studio Build Tools installation is not required.

Python is optional documentation tooling. EngLang checking, running, plotting,
report generation, and packaged execution do not depend on Python.

For user testing and release validation, start with `examples/official`. The
top-level numbered examples are compatibility regressions, while diagnostic and
data-quality fixtures live in separate folders. See
[examples/README.md](examples/README.md).

## Current Commands

```bat
target\debug\eng.exe doctor
target\debug\eng-ide.exe --smoke
target\debug\eng-lsp.exe --smoke
target\debug\eng-ide.exe
target\debug\eng.exe check examples\05_error_messages\unit_mismatch.eng --review
target\debug\eng.exe check examples\05_error_messages\ambiguous_power.eng --review
target\debug\eng.exe entries examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\02_simple_system\main.eng --save-artifacts
target\debug\eng.exe run examples\official\03_integrated_hvac\main.eng --save-artifacts
target\debug\eng.exe run examples\official\17_measured_vs_simulated\main.eng --profile repro --save-artifacts
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
dist\main-standalone\run.bat
target\debug\eng.exe view build\result\result.engres
```

`eng run` lowers through bytecode v1 and the native VM seed. By default,
result, review, report, run-log, process-results, test-results, PlotSpec, SVG,
output manifest, and bytecode payloads are runtime objects in memory. Add
`--save-artifacts` when you want the file set:

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    report_spec.json
    run_log.json
    process_results.json
    test_results.json
    output_manifest.json
    plots/
      plot_spec.json
      plot_manifest.json
      timeseries.svg
```

## Documentation

- [Documentation index](docs/README.md)
- [Current project status](docs/current/status.md)
- [Integrated language philosophy](docs/current/philosophy.md)
- [Version plan](docs/current/version_plan.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [Stable core scope](docs/current/stable_core_scope.md)
- [Development tracks](docs/current/tracks.md)
- [Getting started](docs/development/00_getting_started.md)
- [Repository layout](docs/development/01_repo_layout.md)
- [Daily workflow](docs/development/02_daily_workflow.md)
- [Reproducible environment policy](docs/development/03_environment_reproducibility.md)
- [Native tester IDE](docs/guide/native_ide.md)
- [TimeSeries statistics guide](docs/guide/timeseries_statistics.md)
- [Plotting guide](docs/guide/plotting.md)
- [Report and review artifacts](docs/guide/report_review.md)
- [Run command reference](docs/reference/cli_run.md)
- [Standalone package reference](docs/reference/standalone_package.md)
- [Side effect and general programming policy](docs/reference/side_effect_policy.md)
- [Breaking change policy](docs/reference/breaking_change_policy.md)
- [CLI specification](docs/specs/cli.md)
- [Roadmap](docs/roadmap.md)
- [Release workflow](docs/release/release-workflow.md)

## Core Invariants

- The core execution path must not depend on Python.
- The official lowering direction is `.eng -> typed IR -> bytecode/runtime result objects -> optional .engbc/.engres/PlotSpec/SVG/HTML artifacts`.
- `degC` is the canonical ASCII temperature spelling; `°C` is supported as an
  AbsoluteTemperature alias.
- User-facing execution starts from one `eng.exe`.
- PowerShell scripts are run through the shared `dev.bat` wrapper.
- Public feature claims must match the feature maturity matrix.
- Release versions describe public packages; long-term capabilities are tracked
  as development tracks.
- General programming features must make side effects typed, explicit, and
  reviewable.

## Verification

Before committing a development slice:

```bat
.\dev.bat ci
```

Before a release package check:

```bat
.\dev.bat release-check
pushd dist\englang
eng.exe doctor
eng-ide.exe --smoke
eng-lsp.exe --smoke
eng-ide.exe
eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts
eng.exe run examples\official\02_simple_system\main.eng --save-artifacts
eng.exe run examples\official\03_integrated_hvac\main.eng --save-artifacts
eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
dist\main-standalone\run.bat
popd
```

`package` writes `dist\englang-v1.0.0-windows-x64.zip`, a
matching `.sha256` file, and a curated PDF user guide. The portable package
does not copy the full developer markdown documentation tree. `package-smoke`
extracts that zip into a path with spaces and Korean characters, then runs the
portable `eng.exe`, `eng-ide.exe --smoke`, and `eng-lsp.exe --smoke` without
relying on Rust or Python on the target side. The LSP binary is shipped for
smoke/snapshot tooling; it is not a stable persistent editor-service contract.

`docs-check` and `artifacts-check` are included in `release-check`.
