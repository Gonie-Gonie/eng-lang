# EngLang User Guide

EngLang is a native engineering language for workflows where units, physical
quantity kinds, data schemas, plots, reports, and provenance should be checked
as part of the program. This guide is for the Windows portable package.

The package is intentionally self-contained. A target PC does not need Rust,
Python, Node, Visual Studio Build Tools, or the repository-local development
toolchain.

## Start Here

Extract the zip, open a command prompt in the extracted folder, and run:

```bat
eng.exe doctor
eng-ide.exe --smoke
eng-lsp.exe --smoke
eng.exe run examples/official/03_integrated_hvac/main.eng ^
  --save-artifacts
```

After the final command, open:

```text
build/result/report.html
```

That report is the fastest way to confirm that the runtime, typed CSV import,
unit-aware calculations, PlotSpec/SVG generation, and report writer are all
working from the portable folder.

## Package Contents

The portable folder contains runtime binaries, curated documentation, examples,
standard library seeds, and optional editor tooling.

Runtime binaries:

- `eng.exe`: command-line checker, runner, viewer, formatter, and packager.
- `eng-ide.exe`: native Tauri/WebView IDE for local testing and inspection.
- `eng-lsp.exe`: language-server binary used by editor tooling and smoke checks.
- `WebView2Loader.dll`: required beside `eng-ide.exe`.

Examples:

- `examples/official/`: release-facing examples. Start here.
- `examples/compat/`: compatibility regression fixtures.
- `examples/internal/`: internal solver, uncertainty, ML, domain, and
  component fixtures. These are included for built-in smoke and inspection, but
  they are not public support.
- `examples/diagnostics/`: diagnostic and data-quality fixtures.

Documentation and tooling:

- `stdlib/`: packaged standard library source seeds.
- `docs/EngLang_User_Guide.pdf`: this guide.
- `docs/EngLang_Language_Grammar_Guide.pdf`: grammar and command policy.
- `tools/vscode-englang/`: optional VS Code extension source.
- `tools/englang-vscode-1.0.0.vsix`: optional installable VS Code extension
  package.
- `README.txt`: short package start page.
- `PACKAGE_ASSETS.txt`: portable asset inventory and support boundary.

The package does not include the full developer markdown tree, local build
outputs, repository toolchain caches, Rust sources, Python documentation tools,
or Node development dependencies.

## Official Examples

Use the official examples for user testing and demos. Start with these:

- `examples/official/03_integrated_hvac/main.eng`: integrated HVAC workflow with
  typed CSV promotion, interpolation, constraints, statistics, integration,
  plot, report, and solver metadata.
- `examples/official/17_measured_vs_simulated/main.eng`: measured-vs-simulated
  one-state thermal workflow with `sim.T_zone` and RMSE.
- `examples/official/01_csv_plot/main.eng`: typed CSV import, HeatRate
  calculation, TimeSeries statistics, line PlotSpec, and report output.
- `examples/official/09_command_where_with/main.eng`: command policy, `where`,
  and `with` syntax.
- `examples/official/19_class_object/main.eng`: class declarations, typed
  fields, object literals, and object summaries.

The remaining official examples cover functions/imports, read-only IO, write
output manifests, file policy, structured runtime logs, process-result records,
local test-result records, and histogram PlotSpec output.

Recommended first user test:

```bat
eng.exe run examples/official/03_integrated_hvac/main.eng ^
  --save-artifacts
```

Recommended measured-vs-simulated test:

```bat
eng.exe run examples/official/17_measured_vs_simulated/main.eng ^
  --profile repro --save-artifacts
```

## Native IDE Workflow

Run:

```bat
eng-ide.exe
```

Open `examples/official/03_integrated_hvac/main.eng` from the Explorer, or
create a scratch `.eng` file. Use `Check` for compiler diagnostics and `Run` to
execute the current file. Successful runs update the terminal, Problems tab,
Variables table, PlotSpec preview, Runtime summary, and generated artifact
paths.

The IDE uses the same compiler and runtime crates as `eng.exe`. Diagnostics,
symbols, completions, run artifacts, and report generation therefore exercise
the real core path instead of duplicated editor-only logic.

The Runtime tab summarizes result metadata from `result.engres`, including
policy execution, statistics, solver metadata, generated plots, and package
smoke-oriented inspection data when present.

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
- `build/result/plots/plot_spec.json`: PlotSpec data.
- `build/result/plots/plot_manifest.json`: plot file manifest and hashes.
- `build/result/plots/timeseries.svg`: rendered SVG plot.
- `build/result/run_log.json`: structured runtime log.
- `build/result/process_results.json`: process-result records when used.
- `build/result/test_results.json`: local test-result records when used.
- `build/result/output_manifest.json`: declared output files when used.

For the integrated HVAC example, the result should include policy execution,
computed statistics, integration provenance, plot hashes, and thermal solver
metadata. For the measured-vs-simulated example, the result should include
`sim.T_zone`, time-alignment metadata, RMSE, and a multi-series PlotSpec.

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

The stable-core user contract is intentionally narrow:

- Documented syntax and command policy in the language grammar guide.
- Official examples under `examples/official`.
- Typed CSV import, unit-aware calculations, TimeSeries statistics, local test
  records, process records, structured runtime logs, PlotSpec/SVG/report
  generation, and portable package smoke.
- Native IDE check/run/inspect workflow for the packaged examples.
- Focused one-state thermal workflows documented by the official examples,
  including the measured-vs-simulated `sim.T_zone` case.

These areas are present only as internal or future-facing seeds unless later
documentation marks them stable:

- General nonlinear solving.
- General DAE solving.
- Production multi-domain component graph solving.
- Native JIT execution or speedup claims.
- Broad uncertainty and ML public workflows.
- Full editor platform guarantees beyond the packaged smoke path.

Internal examples remain in the portable package because `eng-ide.exe --smoke`,
`eng-lsp.exe --smoke`, diagnostics, and regression checks use them. Treat those
files as inspection fixtures, not as supported user tutorials.

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
