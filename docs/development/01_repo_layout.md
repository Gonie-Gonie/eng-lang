# Repository Layout

Current public release layout:

```text
.
|-- crates/
|   |-- eng_cli/        user-facing eng.exe commands
|   |-- eng_compiler/   lexer/parser, diagnostics, semantic metadata, bytecode v1
|   |-- eng_ide/        portable Tauri/WebView tester IDE, built as eng-ide.exe
|   |-- eng_jit/        internal hot-kernel detection and lowering-plan metadata
|   |-- eng_lsp/        internal eng-lsp.exe smoke/snapshot/stdio editor service
|   |-- eng_runtime/    run/build/doctor, VM seed, TimeSeries object store, artifacts
|   `-- eng_report/     PlotSpec, SVG plot, HTML review report renderer
|-- docs/
|   |-- user/           first-user guide, tutorials, how-to, concepts
|   |-- reference/      language, stdlib, CLI, diagnostics, artifact lookup
|   |-- workflows/      composite workflow examples and adapter contracts
|   |-- development/    contributor and agent-facing workflow docs
|   |-- internal/       solver, JIT, domain/component, class, runtime internals
|   |-- current/        status, version plan, feature maturity, and tracks
|   |-- architecture/   system and artifact design
|   |-- release/        acceptance checklist and release notes
|   |-- archive/        historical release notes and old long-form plans
|   `-- llm/            compact load maps for future agent work
|-- examples/
|   |-- official/       release-facing examples and manual user-test paths
|   |-- workflows/      composite workflow fixtures
|   |-- internal/       implementation fixtures outside the public contract
|   |-- compat/         older focused regression examples
|   `-- diagnostics/    expected diagnostics and data-quality fixtures
|-- scripts/
|   `-- dev.ps1         the only PowerShell development entry
|-- stdlib/             prelude and unit registry
|-- tools/
|   |-- docs/           optional documentation publishing scripts
|   |-- python/         repo-local Python helper scripts
|   `-- vscode-englang/ optional VS Code extension source
|-- artifacts/docs/     generated release documentation bundles
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
ide-check
jit-plan
entries
run
build
view
test
```

Rules:

```text
- CLI parsing stays dependency-light and std-only for the supported command surface.
- User-facing CLI behavior changes must update docs/reference/cli/spec.md.
- Artifact changes must update docs/architecture/01_runtime_artifacts.md.
```

## `eng_jit`

Plans future native numeric kernels without changing runtime execution.

Current responsibilities:

```text
hot-kernel detection
eng-kernel-plan-v1 JSON
TimeSeries arithmetic candidate detection
TimeSeries statistics fusion candidate detection
TimeSeries integration candidate detection
system residual interface-only candidates
```

Current boundary:

```text
backend = interpreter-fallback
no native code generation
no speedup claim
```

## `eng_compiler`

Checks `.eng` source and emits reviewable compiler metadata.

Current responsibilities:

```text
lexer/parser
source spans
top-level workflow metadata
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
workflow metadata
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
dist/englang
dist/englang-v<version>-windows-x64.zip
dist/englang-v<version>-windows-x64.zip.sha256
dist/englang-user-guide-v<version>.pdf
dist/englang/eng-ide.exe
dist/englang/docs/EngLang_User_Guide.pdf
dist/<model>-standalone/eng.exe
dist/<model>-standalone/run.bat
dist/<model>-standalone/<model>.engpkg
dist/<model>-standalone/<model>.lock
```

Current runtime responsibilities:

```text
top-level workflow file run/build policy
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
