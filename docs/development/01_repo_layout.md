# Repository Layout

Current v1.0-stable layout:

```text
.
|-- crates/
|   |-- eng_cli/        user-facing eng.exe commands
|   |-- eng_compiler/   lexer/parser, diagnostics, semantic/stats/system metadata, bytecode v1
|   |-- eng_ide/        portable native tester IDE, built as eng-ide.exe
|   |-- eng_runtime/    run/build/doctor, VM seed, TimeSeries object store, artifacts
|   `-- eng_report/     PlotSpec, SVG plot, HTML review report renderer
|-- docs/
|   |-- architecture/   system and artifact design
|   |-- development/    setup, workflow, reproducibility
|   |-- master-plan/    source v8/v9 planning documents
|   |-- reference/      command references
|   |-- release/        acceptance checklist and release notes
|   |-- runtime/        bytecode/VM/result contracts
|   |-- tutorials/      step-by-step supported workflow docs
|   |-- user/           curated release-facing user documentation source
|   `-- specs/          CLI and language policy
|-- examples/
|   |-- 01_units/
|   |-- 02_csv_plot/
|   |-- 04_plotting/
|   |-- 05_error_messages/
|   |-- 06_simple_system/
|   `-- 07_data_quality/
|-- scripts/
|   `-- dev.ps1         the only PowerShell development entry
|-- stdlib/             preview prelude and unit registry
|-- tools/
|   |-- python/         repo-local Python requirements for documentation tooling
|   `-- vscode-englang/ optional VS Code extension preview source
|-- dev.bat             common execution-policy bypass wrapper
|-- rust-toolchain.toml pinned Rust toolchain descriptor
`-- Cargo.toml          Rust workspace
```

## `eng_cli`

Builds `eng.exe`.

Current commands:

```text
doctor
new
check
entries
run
build
view
test
```

Rules:

```text
- CLI parsing stays dependency-light and std-only for the preview.
- User-facing behavior changes must update docs/specs/cli.md.
- Artifact changes must update docs/architecture/01_runtime_artifacts.md.
```

## `eng_compiler`

Checks `.eng` source and emits reviewable compiler metadata.

Current responsibilities:

```text
lexer/parser
source spans
script entry metadata
fast `=` declarations
no `:=` diagnostic
dimensionless diagnostics
ambiguous quantity warning
schema and CSV promotion analysis
TimeSeries/statistics metadata and runtime value hooks
system/equation/residual metadata
HeatRate sum lint
physical equation == diagnostic
equation unit consistency diagnostic
entry selection data
bytecode v1 encode/decode
review.json serialization
review_schema_version and table sections
```

Long-term responsibilities:

```text
name resolution
unit/dimension/quantity-kind checking
axis/shape checking
typed IR
function table
bytecode/source map emission
```

## `eng_runtime`

Turns compiler output into run/build artifacts.

Current outputs:

```text
.engbc
.engres
review.json
report.html
report_spec.json
plots/plot_spec.json
plots/plot_manifest.json
plots/timeseries.svg
dist/englang-preview
dist/englang-preview-v<version>-windows-x64.zip
dist/englang-preview-v<version>-windows-x64.zip.sha256
dist/englang-user-test-guide-v<version>.pdf
dist/englang-preview/eng-ide.exe
dist/englang-preview/docs/EngLang_User_Test_Guide.pdf
dist/<model>-standalone/eng.exe
dist/<model>-standalone/run.bat
dist/<model>-standalone/<model>.engpkg
dist/<model>-standalone/<model>.lock
```

Current runtime responsibilities:

```text
entry-required file run/build policy
bytecode decode
VM instruction execution
object store
RuntimeTable CSV column pages
TimeSeries point materialization for the official CSV path
computed statistics/integration payloads
result.engres v1 generation
source/bytecode/data provenance
system residual-only payload metadata
system IR dependency, solver-plan, and solver-boundary payload metadata
packaged standalone runner bundle
```

Long-term responsibilities:

```text
numeric execution
general TimeSeries pages
PlotSpec payloads
portable zip assembly
portable clean-folder smoke
AOT/optimized standalone execution
```

## `eng_report`

Creates reviewable artifacts.

Current outputs:

```text
PlotSpec v1
SVG plot from PlotSpec
plot manifest
ReportSpec v1
system equation summary
HTML review report
```

Long-term responsibilities:

```text
PlotSpec renderer
report spec renderer
review card renderer
unit-aware axis labels
provenance tables
residual summary tables
```

## Core Path Rules

The official `eng.exe run` path must not depend on:

```text
X Python backend
X matplotlib plotting
X global user-machine toolchains
X axis=0/axis=1 public APIs
```

Development-time release documentation may use the repo-local portable Python
environment installed by `dev.bat setup`. That tooling must stay outside the
core `eng.exe run` path and outside target-PC package requirements.

Use `dev.bat` for development tasks. Do not add extra PowerShell entry scripts unless they are routed through the shared wrapper.
