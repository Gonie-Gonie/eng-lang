# EngLang

EngLang is a native programming language project for engineering simulation workflows. Its goal is to let the compiler and runtime understand units, physical quantity kinds, schemas, axes, statistics, plotting, reports, and provenance as first-class parts of engineering code.

The current repository follows the v9 master plan. The v9 change keeps the v8 language decisions, including fast `=` declarations and no `:=`, but reorganizes development around a version-by-version execution roadmap from `v0.1-preview` through `v2.0`.

## Quick Start

On Windows, use the root `dev.bat` wrapper for all development commands. It bypasses PowerShell execution-policy issues and keeps the toolchain local to the repository.

```bat
.\dev.bat setup
.\dev.bat doctor
.\dev.bat ci
.\dev.bat run-example
```

`setup` installs the pinned Rust toolchain into `.dev`, fetches dependencies, and builds the workspace. A global Rust installation and Python are not required for the core preview path.

## Current Preview Commands

```bat
target\debug\eng.exe doctor
target\debug\eng.exe check examples\05_error_messages\unit_mismatch.eng --review
target\debug\eng.exe check examples\05_error_messages\ambiguous_power.eng --review
target\debug\eng.exe run examples\04_plotting\main.eng
target\debug\eng.exe build examples\04_plotting\main.eng --standalone --profile repro
target\debug\eng.exe view build\result\result.engres
```

`eng run` generates preview artifacts even before the full VM is implemented:

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    plots/timeseries.svg
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
```

Active planning target:

```text
v0.3-preview
  Schema, promote, CSV data boundary, schema symbol table, source file
  provenance, CSV diagnostics.
```

## Documentation

- [Documentation index](docs/README.md)
- [Getting started](docs/development/00_getting_started.md)
- [Repository layout](docs/development/01_repo_layout.md)
- [Daily workflow](docs/development/02_daily_workflow.md)
- [Reproducible environment policy](docs/development/03_environment_reproducibility.md)
- [Version roadmap workflow](docs/development/04_version_roadmap_workflow.md)
- [System architecture](docs/architecture/00_system_overview.md)
- [Compiler frontend](docs/architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](docs/architecture/03_expected_types_and_quantities.md)
- [CLI specification](docs/specs/cli.md)
- [v8/v9 language policy](docs/specs/language-v8.md)
- [Fast assignment guide](docs/language/fast_assignment.md)
- [Dimensionless policy guide](docs/language/dimensionless.md)
- [Roadmap](docs/roadmap.md)
- [v9 master plan](docs/master-plan/EngLang_LongTerm_Development_Master_Plan_v9.md)
- [v8 to v9 revision guide](docs/master-plan/EngLang_v8_to_v9_Revision_Guide.md)

## Core Invariants

- The core execution path must not depend on Python.
- The official lowering direction is `.eng -> typed IR -> .engbc -> eng runtime -> .engres -> PlotSpec -> SVG/HTML review artifacts`.
- User-facing execution starts from one `eng.exe`.
- PowerShell scripts are run through the shared `dev.bat` wrapper.
- Public features must include examples, tests, and reviewable artifacts.
- Work should target a specific roadmap version and pass that version's release gate.

## Verification

Before committing a development slice:

```bat
.\dev.bat ci
```

Before a preview package check:

```bat
.\dev.bat package
dist\englang-preview\eng.exe doctor
dist\englang-preview\eng.exe run examples\04_plotting\main.eng
```
