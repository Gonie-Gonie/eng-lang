# EngLang

EngLang is a native programming language project for engineering simulation workflows. Its goal is to let the compiler and runtime understand units, physical quantity kinds, schemas, axes, statistics, plotting, reports, and provenance as first-class parts of engineering code.

The current repository follows the v9 master plan. For day-to-day work, start
from the current-status layer instead of loading every historical planning
document:

- [Current project status](docs/current/status.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [v1.0.3 hardening register](docs/current/v1_0_3_hardening.md)
- [LLM context](LLM_CONTEXT.md)
- [LLM load map](docs/llm/load_map.yml)

The current stable baseline is `v1.0-stable`. The active release target is
`v1.0.3`, focused on native IDE and documentation hardening. `v1.1`
uncertainty and `v1.2` data-driven modeling code may exist on `main`, but those
features are experimental until their language rules, runtime behavior,
diagnostics, IDE metadata, examples, tests, and user documentation are aligned.

## Quick Start

On Windows, use the root `dev.bat` wrapper for all development commands. It bypasses PowerShell execution-policy issues and keeps the toolchain local to the repository.

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
data-quality fixtures live in separate folders. See [examples/README.md](examples/README.md).

## Current Stable Commands

```bat
target\debug\eng.exe doctor
target\debug\eng-ide.exe --smoke
target\debug\eng-ide.exe
target\debug\eng.exe check examples\05_error_messages\unit_mismatch.eng --review
target\debug\eng.exe check examples\05_error_messages\ambiguous_power.eng --review
target\debug\eng.exe entries examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\02_simple_system\main.eng
target\debug\eng.exe run examples\official\03_integrated_hvac\main.eng
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main --input data/sensor.csv
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat
target\debug\eng.exe view build\result\result.engres
```

`eng run` now lowers through bytecode v1 and the native VM seed:

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

## Development Milestones

Completed and pushed:

```text
v0.1-preview
  Repository bootstrap, CLI skeleton, parser/frontend foundation, unit seed,
  runtime artifact skeleton, docs, CI wrapper.

v0.2-preview
  Expected type skeleton, quantity completion table, hover data, refined
  dimensionless and ambiguous quantity diagnostics.

v0.3-preview
  Schema symbol table, promote csv validation, CSV header checks, source file
  hash provenance, missing policy/constraint seed metadata.

v0.4-preview
  Bytecode v1, entry-based run, VM object store seed, scalar/table runtime
  values, result.engres v1 typed payload, entry listing and missing-entry error.

v0.5-preview
  TimeSeries[Time] inference, axis metadata, statistics summary metadata,
  computed mean/time_weighted_mean/median/std/p90/p95 values for the official
  CSV path, trapezoidal integrate provenance, HeatRate sum lint.

v0.6-preview
  PlotSpec v1, line plot data model, unit-aware axis labels, SVG rendering
  from PlotSpec, plot manifest, `eng view` plot listing.

v0.7-alpha
  Review schema hardening, ReportSpec v1, variable table, inferred declaration
  table, unit conversion table, schema summary, plot manifest section, warning
  list, and report_spec_hash provenance.

v0.8-alpha
  Minimal system/equation support: system block, parameter/state/input,
  equation block, infix eq relation, der(), unit consistency diagnostics,
  residual metadata, report/review system summaries, system IR dependencies,
  solver boundary metadata, and a fixed-step ODE preview for the official
  simple thermal system.

v0.9-alpha
  Portable demo hardening: Windows zip package, SHA256 checksum,
  package-smoke extraction under Korean and space-containing paths,
  official CSV+plot and simple system examples, and no install-required
  preview execution.

v1.0-stable
  Stable core release: typed CSV boundary, unit/quantity calculations,
  row-level CSV runtime pages, TimeSeries statistics, CSV-derived PlotSpec
  SVG/report, schema policy execution status, minimal system/equation metadata,
  explicit solver-boundary artifacts, Args help/flag binding metadata, and
  runnable packaged standalone bundles.

v1.0.1 / v1.0.2 internal hardening notes
  Native portable tester IDE, integrated HVAC user-test example, PDF user guide,
  VS Code extension preview, and package-smoke IDE validation were developed on
  main as preparation for the next public hardening release. Treat these as
  internal notes, not the active public release target.
```

Active planning target:

```text
v1.0.3
  Native IDE quality hardening before the next release: VS Code-like explorer,
  file creation, syntax highlighting, lint/problem surfacing, clearer
  completions/symbol inspection, in-IDE PlotSpec preview, runtime artifact
  summaries for uncertainty/ML user tests, curated user PDF docs, and
  repo-local portable Python/oodocs setup.

v1.1
  Uncertainty core: Measured[T], Interval[T], distribution/ensemble seeds,
  uncertainty metadata, simple propagation, and uncertainty report summaries.
  Experimental on main until v1.1 is explicitly targeted and released.

v1.2
  Data-driven modeling seed: eng.ml preview surface, train/test split,
  regression/basic MLP, RMSE/MAE/R2, leakage lint, model card, and parity plot.
  Experimental on main until v1.2 is explicitly targeted and released.
```

## Documentation

- [Documentation index](docs/README.md)
- [Current project status](docs/current/status.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [v1.0.3 hardening register](docs/current/v1_0_3_hardening.md)
- [LLM context](LLM_CONTEXT.md)
- [LLM load map](docs/llm/load_map.yml)
- [Current master plan pointer](docs/master-plan/current.md)
- [Getting started](docs/development/00_getting_started.md)
- [Repository layout](docs/development/01_repo_layout.md)
- [Daily workflow](docs/development/02_daily_workflow.md)
- [Reproducible environment policy](docs/development/03_environment_reproducibility.md)
- [Version roadmap workflow](docs/development/04_version_roadmap_workflow.md)
- [System architecture](docs/architecture/00_system_overview.md)
- [Bytecode VM and result v1](docs/runtime/bytecode.md)
- [TimeSeries statistics guide](docs/guide/timeseries_statistics.md)
- [Plotting guide](docs/guide/plotting.md)
- [Uncertainty core guide](docs/guide/uncertainty.md)
- [Data-driven modeling guide](docs/guide/data_driven_modeling.md)
- [Native tester IDE](docs/guide/native_ide.md)
- [Report and review artifacts](docs/guide/report_review.md)
- [Run command reference](docs/reference/cli_run.md)
- [CLI specification](docs/specs/cli.md)
- [v8/v9 language policy](docs/specs/language-v8.md)
- [Roadmap](docs/roadmap.md)
- [Release workflow](docs/release/release-workflow.md)

## Core Invariants

- The core execution path must not depend on Python.
- The official lowering direction is `.eng -> typed IR -> .engbc -> eng runtime -> .engres -> PlotSpec -> SVG/HTML review artifacts`.
- `degC` is the canonical ASCII temperature spelling; `°C` is supported as an
  AbsoluteTemperature alias.
- User-facing execution starts from one `eng.exe`.
- PowerShell scripts are run through the shared `dev.bat` wrapper.
- Public features must include examples, tests, and reviewable artifacts.
- Work should target a specific roadmap version and pass that version's release gate.

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
eng-ide.exe
eng.exe run examples\official\01_csv_plot\main.eng --entry main
eng.exe run examples\official\02_simple_system\main.eng --entry main
eng.exe run examples\official\03_integrated_hvac\main.eng --entry main
eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat
popd
```

`package` writes `dist\englang-preview-v<version>-windows-x64.zip`, a matching
`.sha256` file, and a curated PDF user guide. The portable package does not copy
the full developer markdown documentation tree. `package-smoke` extracts that
zip into a path with spaces and Korean characters, then runs the portable
`eng.exe` and `eng-ide.exe --smoke` without relying on Rust or Python on the
target side. It also builds and runs the standalone packaged runner from inside
the extracted portable package.

`docs-check` and `artifacts-check` are included in `release-check`.
`docs-check` validates supported `eng` documentation snippets. `artifacts-check`
validates the official example artifact schemas and golden baselines.
