# EngLang

EngLang is a semantic engineering workflow language.
It helps engineers and LLM-generated code preserve units, quantities, schemas,
axes, provenance, plots, and review artifacts across typed data analysis and
simulation-result validation.

EngLang is not a solver-first language. Scoped simulation paths can produce
typed TimeSeries and reviewable solver artifacts, but the public identity is
unit-safe engineering computation that humans and LLMs can inspect.

## Status

Current public line: `v0.1.0`

Active target: `v0.1.x` - repo cleanup and scoped additions

Workspace package version: `0.1.0`

EngLang `0.1.0` is a clean initial portable release. The documented
data-to-report workflow, artifact family, packaged runner, and native tester
path are the public contract. Internal implementation seeds remain outside that
contract unless a status document gives a narrow supported scope.

| EngLang is | EngLang is not |
|---|---|
| A semantic engineering workflow language | A complete engineering solver platform |
| Unit-safe typed data analysis | A Modelica or Simulink replacement |
| TimeSeries, schema, axis, and provenance semantics | An EnergyPlus replacement |
| Report/review artifacts for engineering computation | A production multi-domain simulator |
| IDE inspection for variables, units, schemas, TimeSeries, and artifacts | A general nonlinear/DAE solver release |

Start from these short status documents:

- [Current project status](docs/current/status.md)
- [Integrated language philosophy](docs/current/philosophy.md)
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [Development tracks](docs/current/tracks.md)
- [Uncertainty track](docs/current/uncertainty.md)
- [Reviewability track](docs/current/reviewability.md)
- [Composite workflow base modules](docs/current/workflow_modules.md)
- [Version plan](docs/current/version_plan.md)
- [LLM context](LLM_CONTEXT.md)
- [LLM load map](docs/llm/load_map.yml)

## Public Package Workflows

The public package is organized around six workflow groups:

1. Typed data boundary through schemas, CSV promote, source hashes, and
   provenance-visible input metadata.
2. Unit/quantity-aware TimeSeries calculation, statistics, and integration.
3. Plot, report, review, and artifact generation for engineering results.
4. Measured-vs-simulated validation with explicit TimeSeries inputs, metrics,
   time-alignment metadata, and reviewable plots.
5. Explicit side-effect scripting for paths, reads, writes, logs, process
   results, output manifests, and local test artifacts.
6. Portable package execution and a native tester IDE for artifact inspection.

System/equation examples are scoped supporting capability. They are useful when
they produce typed TimeSeries, residual evidence, convergence metadata, and
reviewable failure artifacts. Detailed solver status belongs in
[solver track docs](docs/solver/README.md), not in the README first screen.

## Quick Start

On Windows, use the root `dev.bat` wrapper for development commands. It
bypasses PowerShell execution-policy issues and keeps the toolchain local to
the repository.

```bat
.\dev.bat setup
.\dev.bat doctor
.\dev.bat ci
.\dev.bat docs-check
.\dev.bat artifacts-check
.\dev.bat run-example
```

`setup` installs the pinned Rust toolchain, repo-local MinGW GNU build support,
and a portable Python documentation toolchain into `.dev`, fetches
dependencies, and builds the workspace. A global Rust, Python, MinGW, or Visual
Studio Build Tools installation is not required.

Python is optional documentation tooling. EngLang checking, running, plotting,
report generation, and packaged execution do not depend on Python.

For user testing and release validation, start with the core workflow examples:

- `examples/official/01_csv_plot`
- `examples/official/09_command_where_with`
- `examples/official/16_test_assert_golden`

Composite workflow examples live under `examples/workflows`. They demonstrate
API/file/process/model/report pipelines through generic adapter boundaries, not
domain-specific core language claims.

Solver-heavy examples remain in the repository as advanced/internal smoke
fixtures until their paths can be moved without breaking package and IDE gates.
See [examples/README.md](examples/README.md).

## Current Commands

```bat
target\debug\eng.exe doctor
target\debug\eng-ide.exe --smoke
target\debug\eng.exe check examples\diagnostics\error_messages\unit_mismatch.eng --review
target\debug\eng.exe entries examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\09_command_where_with\main.eng --save-artifacts
target\debug\eng.exe run examples\official\16_test_assert_golden\main.eng --save-artifacts
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
dist\main-standalone\run.bat
target\debug\eng.exe view build\result\result.engres
```

`eng-lsp.exe --smoke` remains available for internal snapshot/editor tooling.
It is not a stable persistent editor-service contract.

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
- [Feature maturity matrix](docs/current/feature_maturity_matrix.md)
- [Development tracks](docs/current/tracks.md)
- [Semantic benchmark strategy](benchmarks/README.md)
- [Native tester IDE](docs/guide/native_ide.md)
- [TimeSeries statistics guide](docs/guide/timeseries_statistics.md)
- [Plotting guide](docs/guide/plotting.md)
- [Report and review artifacts](docs/guide/report_review.md)
- [Run command reference](docs/reference/cli_run.md)
- [Standalone package reference](docs/reference/standalone_package.md)
- [Side effect and general programming policy](docs/reference/side_effect_policy.md)
- [Breaking change policy](docs/reference/breaking_change_policy.md)
- [CLI specification](docs/specs/cli.md)
- [Release workflow](docs/release/release-workflow.md)

## Core Invariants

- The core execution path must not depend on Python.
- The official lowering direction is `.eng -> typed IR -> bytecode/runtime result objects -> optional .engbc/.engres/PlotSpec/SVG/HTML artifacts`.
- `degC` is the canonical ASCII temperature spelling.
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
eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts
eng.exe run examples\official\09_command_where_with\main.eng --save-artifacts
eng.exe run examples\official\16_test_assert_golden\main.eng --save-artifacts
eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
dist\main-standalone\run.bat
popd
```

`package` writes `dist\englang-v0.1.0-windows-x64.zip`, a matching `.sha256`
file, and a curated PDF user guide. The portable package does not copy the full
developer markdown documentation tree. `package-smoke` extracts that zip into a
path with spaces and Korean characters, then runs the portable `eng.exe`,
`eng-ide.exe --smoke`, and internal LSP smoke without relying on Rust or Python
on the target side.

`docs-check` and `artifacts-check` are included in `release-check`.
