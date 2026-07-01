# EngLang User Guide

EngLang is a semantic engineering workflow language for programs where units,
physical quantity kinds, data schemas, TimeSeries axes, plots, reports, and
provenance should be checked as part of the computation. This guide is for the
Windows portable package.

The package is intentionally self-contained. A target PC does not need Rust,
Python, Node, Visual Studio Build Tools, or the repository-local development
toolchain.

## Start Here

Extract the zip, open a command prompt in the extracted folder, and run:

```bat
eng.exe doctor
eng-ide.exe --smoke
eng.exe run examples/official/01_csv_plot/main.eng ^
  --save-artifacts
```

After the final command, open:

```text
build/result/report.html
```

That report is the fastest way to confirm that the runtime, typed CSV import,
unit-aware calculations, PlotSpec/SVG generation, review metadata, and report
writer are working from the portable folder.

`eng-lsp.exe --smoke` is also bundled for internal editor-service snapshot
checks. It is not a stable persistent editor-service contract.

## Package Contents

Runtime binaries:

- `eng.exe`: command-line checker, runner, viewer, formatter, and packager.
- `eng-ide.exe`: native Tauri/WebView IDE for local testing and inspection.
- `eng-lsp.exe`: language-server binary used by editor tooling and smoke
  checks.
- `WebView2Loader.dll`: required beside `eng-ide.exe`.

Examples:

- `examples/official/`: package smoke examples. Start with the core workflow
  examples listed below.
- Advanced solver, compatibility, diagnostic, and internal regression examples
  remain in the source repository. They are not bundled as portable package
  tutorials.

Documentation and tooling:

- `stdlib/`: packaged standard library module source files.
- `docs/EngLang_User_Guide.pdf`: this guide.
- `docs/EngLang_Language_Grammar_Guide.pdf`: grammar and command policy.
- `tools/vscode-englang/`: optional VS Code extension source.
- `tools/englang-vscode-0.1.0.vsix`: optional installable VS Code extension.
- `README.txt`: short package start page.
- `PACKAGE_ASSETS.txt`: portable asset inventory and support boundary.

The package does not include the full developer markdown tree, local build
outputs, repository toolchain caches, Rust sources, Python documentation tools,
or Node development dependencies.

## Core Workflow Examples

Use these examples for first user testing and demos:

- `examples/official/01_csv_plot/main.eng`: typed CSV import, HeatRate
  calculation, TimeSeries statistics, integration metadata, line PlotSpec, and
  report output.
- `examples/official/07_functions_imports/main.eng`: imports, const values,
  scalar functions, unit checks, print, and summary export.
- `examples/official/08_print_export_summary/main.eng`: compact unit-aware
  print and summary CSV export.
- `examples/official/09_command_where_with/main.eng`: command policy, `where`,
  and `with` syntax.
- `examples/official/10_path_policy/main.eng`: typed paths and environment
  dependency provenance.
- `examples/official/11_read_only_io/main.eng`: read-only text/json/toml input
  provenance.
- `examples/official/12_write_output_manifest/main.eng`: explicit write outputs
  and output manifest records.
- `examples/official/14_run_log/main.eng`: structured runtime log artifacts.
- `examples/official/15_process_result/main.eng`: explicit process-result
  artifacts.
- `examples/official/16_test_assert_golden/main.eng`: local test/assert/golden
  artifacts.

Additional review examples:

- `examples/official/13_file_operations/main.eng`: constrained file operation
  metadata.
- `examples/official/19_class_object/main.eng`: typed engineering objects,
  validation, object summaries, and IDE/LSP metadata.

Solver-heavy examples under `examples/advanced_solver` are advanced smoke
examples. They are useful for inspecting typed TimeSeries and solver artifacts,
but they are not a broad solver platform claim and should not be the first
walkthrough.

Recommended first user test:

```bat
eng.exe run examples/official/01_csv_plot/main.eng ^
  --save-artifacts
```

## Native IDE Workflow

Run:

```bat
eng-ide.exe
```

Open `examples/official/01_csv_plot/main.eng` from the Explorer, or create a
scratch `.eng` file. Use `Check` for compiler diagnostics and `Run` to execute
the current file. Successful runs update the terminal, Problems tab, Variables
table, Schema panel, TimeSeries panel, PlotSpec preview, Runtime summary, and
generated artifact paths.

The IDE uses the same compiler and runtime crates as `eng.exe`. Diagnostics,
symbols, completions, run artifacts, and report generation therefore exercise
the real core path instead of duplicated editor-only logic.

The IDE is best understood as an engineering review cockpit: variables, units,
schemas, TimeSeries, metrics, validations, reports, review JSON, output
manifests, run logs, process results, and test results are inspectable without
opening raw artifact files first.

## Command-Line Workflow

Check the package environment:

```bat
eng.exe doctor
```

Check one file:

```bat
eng.exe check examples/official/01_csv_plot/main.eng
```

Run one file and save artifacts:

```bat
eng.exe run examples/official/01_csv_plot/main.eng ^
  --save-artifacts
```

View a result record:

```bat
eng.exe view build/result/result.engres
```

Build and run a standalone bundle:

```bat
eng.exe build examples/official/01_csv_plot/main.eng ^
  --standalone --profile repro
dist/main-standalone/run.bat
```

Generated run artifacts are written under `build/result` in the current working
folder.

## Expected Artifacts

A successful run can create:

- `build/result/result.engres`: main structured result record.
- `build/result/review.json`: review and policy metadata.
- `build/result/report_spec.json`: report input model.
- `build/result/report.html`: human-readable report.
- `build/result/run_plan.json`: workflow graph and node status artifact.
- `build/result/plots/plot_spec.json`: PlotSpec data.
- `build/result/plots/plot_manifest.json`: plot file manifest and hashes.
- `build/result/plots/timeseries.svg`: rendered SVG plot.
- `build/result/run_log.json`: structured runtime log.
- `build/result/process_results.json`: process-result records when used.
- `build/result/test_results.json`: local test-result records when used.
- `build/result/output_manifest.json`: declared output files when used.

For the CSV/plot example, the result should include typed CSV provenance,
computed statistics, integration provenance, plot hashes, and report metadata.

## Standalone Bundle

Build:

```bat
eng.exe build examples/official/01_csv_plot/main.eng ^
  --standalone --profile repro
```

Run:

```bat
dist/main-standalone/run.bat --help
dist/main-standalone/run.bat
```

The standalone bundle contains the runner, compiled package files, lock/review
metadata, source snapshot, and required source data. It writes normal
`build/result` artifacts inside the standalone folder.

## Current Boundaries

The public user contract is intentionally narrow:

- Documented syntax and command policy in the language grammar guide.
- Core workflow examples under `examples/official`.
- Typed CSV import, unit-aware calculations, TimeSeries statistics, local test
  records, process records, structured runtime logs, PlotSpec/SVG/report
  generation, and portable package smoke.
- Native IDE check/run/inspect workflow for packaged examples.

These areas are present only as internal or future-facing implementation tracks
unless current documentation marks them stable or supported for a narrow scope:

- Broad public solver support outside the documented scoped examples.
- General nonlinear solving.
- General DAE solving.
- Production multi-domain component graph solving.
- Native JIT execution or speedup claims.
- Broad uncertainty and ML public workflows.
- Full editor platform guarantees beyond the packaged smoke path.

Internal examples, including system/solver examples, remain in the portable
package because `eng-ide.exe --smoke`, internal LSP smoke, diagnostics, and
regression checks use them. Treat those files as inspection-only examples, not
as supported user tutorials.

## Troubleshooting

If `eng.exe doctor` fails, run it from the extracted package root so it can find
`examples`, `stdlib`, and writeable output folders.

If an example run fails, run:

```bat
eng.exe check <path-to-file.eng>
```

Then inspect the Problems list in the IDE or the command-line diagnostics.

If a CSV path fails, keep relative paths anchored next to the source file, as in
the official examples.

If a report does not open automatically, open:

```text
build/result/report.html
```

If the IDE does not start, confirm that `WebView2Loader.dll` is next to
`eng-ide.exe` and that the extracted folder was not partially copied.
