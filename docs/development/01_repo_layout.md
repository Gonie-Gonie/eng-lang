# Repository Layout

Current public release layout:

```text
.
|-- crates/
|   |-- eng_cli/        user-facing eng.exe commands
|   |-- eng_compiler/   lexer/parser, diagnostics, semantic metadata, bytecode v1
|   |-- eng_ide/        portable native IDE shell, built as eng-ide.exe
|   |-- eng_jit/        internal hot-kernel detection and lowering-plan metadata
|   |-- eng_lsp/        internal eng-lsp.exe smoke/snapshot/stdio editor service
|   |-- eng_runtime/    bytecode VM, native workflows, solvers, runtime artifacts
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
|   |-- workflows/      composite native workflow programs
|   |-- internal/       implementation fixtures outside the public contract
|   |-- compat/         older focused regression examples
|   `-- diagnostics/    expected diagnostics and data-quality fixtures
|-- scripts/
|   `-- dev.ps1         the only PowerShell development entry
|-- stdlib/             prelude and unit registry
|-- tools/
|   |-- docs/           optional documentation publishing scripts
|   |-- python/         optional documentation publishing requirements
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

## `eng_compiler`

Owns the source-to-semantic contract for `.eng` programs.

Current responsibilities:

```text
lexer, parser, AST, formatter, and source spans
imports, modules, declarations, expected types, and symbol metadata
unit, dimension, quantity-kind, axis, and supported shape checks
schema, table, TimeSeries, statistics, and plotting contracts
native net/cache/sampling/case/db/model/uncertainty workflow contracts
system, component, equation, residual, and behavior graph metadata
diagnostics, hover/review metadata, and compiler quick fixes
bytecode v1 encoding/decoding and review.json serialization
```

Supported subsets and remaining gaps are tracked in
[feature maturity](../current/feature_maturity_matrix.md) and the
[internal solver plan](../internal/solver/generic_solver_completion_plan.md);
the layout document does not duplicate those roadmaps.

## `eng_runtime`

Executes compiler output and produces native runtime values and artifact
payloads.

Current responsibilities:

```text
top-level workflow file run/build policy
bytecode decode and VM instruction execution
typed scalar, object, table, and TimeSeries materialization
native workflow execution for the module-registry-supported surface
case scheduling, cache replay, model/DB, and uncertainty runtime data
linear, fixed-point, Newton, ODE, state-space, DAE, component, and behavior
  solver subsets documented by their examples and limitation artifacts
result.engres, review/report payloads, logs, manifests, and provenance
standalone and portable package runtime assembly
```

## `eng_report`

Renders compiler/runtime metadata into reviewable visual artifacts.

```text
PlotSpec v1
SVG plots and plot manifests
ReportSpec v1
HTML review report
unit-aware axis labels
provenance, workflow, equation, residual, convergence, and failure tables
```

## `eng_lsp`

Adapts compiler-owned editor semantics to the Language Server Protocol.

```text
persistent stdio service and deterministic snapshot mode
diagnostics, hover, completion, semantic tokens, and formatting
definition, references, rename, symbols, highlights, and code actions
UTF-16 range conversion, cancellation, and stale-result suppression
```

## `eng_ide`

Packages the native Tauri IDE shell and its static frontend.

```text
compiler check/run/report service bridge
source editor, diagnostics, variables, artifacts, plots, and assembly panels
source-span navigation and canonical user-facing status labels
portable target-PC smoke without Node, Rust, or Python
```

## `eng_jit`

Produces native-kernel candidate plans without changing runtime execution.

```text
hot-kernel detection and eng-kernel-plan-v1 JSON
TimeSeries arithmetic/statistics/integration candidates
system residual and component solver kernel candidates
interpreter-fallback parity and benchmark metadata
no native code generation or speedup claim
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

Use `dev.bat` for development tasks. Do not add extra PowerShell entry
scripts unless they are routed through the shared wrapper.
