# EngLang

EngLang is a native programming language project for engineering simulation
workflows. Its goal is to let the compiler and runtime understand units,
physical quantity kinds, schemas, axes, statistics, plotting, reports, and
provenance as first-class parts of engineering code.

## Status

Current public line: `v0.1-preview`

Active target: `v0.2-preview` - IDE and documentation hardening

EngLang is preview software. The language, runtime behavior, and artifact
formats are not yet stable.

Start from these short status documents:

- [Current project status](docs/current/status.md)
- [Version plan](docs/current/version_plan.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [Development tracks](docs/current/tracks.md)
- [LLM context](LLM_CONTEXT.md)
- [LLM load map](docs/llm/load_map.yml)

## Supported Preview Workflows

- Typed CSV promote through official examples
- Unit-aware TimeSeries calculations
- TimeSeries statistics and integration metadata
- PlotSpec/SVG output
- Review/report artifacts
- Basic packaged execution
- Native tester IDE for user testing

Future work is managed by tracks, not by high-numbered public versions:

- Core language
- Data boundary
- Statistics, plot, and report
- System/equation
- IDE/LSP
- Uncertainty
- Data-driven modeling
- Runtime optimization/JIT/AOT
- Domain/component

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

`setup` installs the pinned Rust toolchain and a portable Python documentation
toolchain into `.dev`, fetches dependencies, and builds the workspace. A global
Rust or Python installation is not required.

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
target\debug\eng.exe run examples\official\02_simple_system\main.eng
target\debug\eng.exe run examples\official\03_integrated_hvac\main.eng
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat
target\debug\eng.exe view build\result\result.engres
```

`eng run` lowers through bytecode v1 and the native VM seed:

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    report_spec.json
    plots/
      plot_spec.json
      plot_manifest.json
      timeseries.svg
```

## Documentation

- [Documentation index](docs/README.md)
- [Current project status](docs/current/status.md)
- [Version plan](docs/current/version_plan.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
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
- [CLI specification](docs/specs/cli.md)
- [Roadmap](docs/roadmap.md)
- [Release workflow](docs/release/release-workflow.md)

## Core Invariants

- The core execution path must not depend on Python.
- The official lowering direction is `.eng -> typed IR -> .engbc -> eng runtime -> .engres -> PlotSpec -> SVG/HTML review artifacts`.
- `degC` is the canonical ASCII temperature spelling; `°C` is supported as an
  AbsoluteTemperature alias.
- User-facing execution starts from one `eng.exe`.
- PowerShell scripts are run through the shared `dev.bat` wrapper.
- Public feature claims must match the feature maturity matrix.
- Release versions describe public packages; long-term capabilities are tracked
  as development tracks.

## Verification

Before committing a development slice:

```bat
.\dev.bat ci
```

Before a release package check:

```bat
.\dev.bat release-check
pushd dist\englang-preview
eng.exe doctor
eng-ide.exe --smoke
eng-lsp.exe --smoke
eng-ide.exe
eng.exe run examples\official\01_csv_plot\main.eng --entry main
eng.exe run examples\official\02_simple_system\main.eng --entry main
eng.exe run examples\official\03_integrated_hvac\main.eng --entry main
eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat
popd
```

`package` writes `dist\englang-preview-v0.1-preview-windows-x64.zip`, a
matching `.sha256` file, and a curated PDF user guide. The portable package
does not copy the full developer markdown documentation tree. `package-smoke`
extracts that zip into a path with spaces and Korean characters, then runs the
portable `eng.exe`, `eng-ide.exe --smoke`, and experimental `eng-lsp.exe
--smoke` without relying on Rust or Python on the target side.

`docs-check` and `artifacts-check` are included in `release-check`.
