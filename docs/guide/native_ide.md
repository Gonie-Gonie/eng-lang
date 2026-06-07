# Native Tester IDE

The v1.0.3 hardening path upgrades the portable native tester IDE. It is built as
`eng-ide.exe` from the Rust workspace and is shipped inside the portable Windows
zip beside `eng.exe`.

The goal is practical user testing before v1.1 uncertainty work starts:

```text
- browse official examples, stdlib, and tutorial sources
- create new .eng files
- edit source in a native GUI
- see syntax highlighting and line-level diagnostic tinting
- run compiler diagnostics while editing
- inspect quantity/unit symbol metadata
- use keyword, quantity, unit, and snippet completions
- run the current file, preview PlotSpec data, inspect runtime summaries, and
  open generated artifacts
```

This tester IDE is intentionally native Rust GUI code using `eframe`/`egui`.
It does not require a browser, Python, Node, Rust, or Visual Studio Build Tools
on a target user PC after the portable package has been downloaded.

## Portable User Flow

Download and extract the EngLang portable zip, then run:

```bat
eng-ide.exe
```

For a non-GUI smoke check:

```bat
eng-ide.exe --smoke
```

The smoke path checks that the IDE can discover example files and call the same
compiler metadata path used by `eng.exe check`.

On Windows, `eng-ide.exe` is built as a GUI subsystem binary so launching it from
Explorer does not create a separate console window.

## Development Flow

From the repository root:

```bat
.\dev.bat setup
.\dev.bat ide --smoke
.\dev.bat ide
```

`dev.bat ide` runs the native GUI through Cargo. It uses the same pinned
repository-local Rust toolchain as the rest of the project.

## Interface

The native IDE follows a familiar editor layout:

```text
Top toolbar
  Check, Save, Run, Report, Plot SVG, entry selection, diagnostic counts,
  Explorer/Sidebar/Result visibility toggles, dirty state, and current
  status.

Left Explorer
  Opens .eng files from examples/, stdlib/, and selected tutorial sources.
  Creates scratch .eng files from a starter template. Open File and Open Folder
  use native OS dialogs.

Main work area
  The center is split as Code on the left and Result on the right. The Result
  panel is resizable and independently scrollable, so plots, runtime summaries,
  and artifact links can be reviewed without leaving the editor.

Code
  Native multiline editor with EngLang syntax highlighting and line-level
  diagnostic backgrounds. The editor uses a larger monospace style and expands
  to the available center width.

Result
  Run Preview renders PlotSpec points inside the IDE with axes, grid lines,
  ticks, and plot-specific rendering for line, scatter, bar, and histogram
  plots. Runtime Summary and Artifacts are shown below the plot in the same
  scrollable result panel.

Right Sidebar
  Tabbed Symbols, Completions, and Runtime Summary surface. After Run, the
  Runtime tab shows result status, uncertainty summaries, ML metrics,
  coefficients, loss history, policy count, and system count.

Bottom panel
  Problems, Output, and Artifacts tabs.
```

Diagnostics are produced by `eng_compiler::check_source`, so unsaved edits can
be checked without writing temporary source files. Running a program saves the
current file first and then calls the same runtime path as:

```bat
eng.exe run <file.eng> --entry main
```

Generated runtime artifacts are written under:

```text
build/ide-run/
```

After a successful run, the Artifacts tab shows:

```text
report.html
report_spec.json
plots/timeseries.svg
plots/plot_spec.json
plots/plot_manifest.json
result.engres
review.json
main.engbc
```

Recommended release user-test example:

```text
examples/official/03_integrated_hvac/main.eng
```

See [Integrated HVAC user test](../tutorials/06_integrated_hvac.md) for the
step-by-step flow.

Recommended v1.1 uncertainty smoke:

```text
examples/official/04_uncertainty_core/main.eng
```

Run it and inspect the Plot SVG/Report artifacts to verify the in-report
histogram and Uncertainty table. The Runtime Summary tab also shows
distribution kind, propagation method, sample count, and p05/p50/p95 values.

Recommended v1.2 data-driven modeling smoke:

```text
examples/official/05_data_driven_modeling/main.eng
```

Run it and inspect the Run Preview/Plot SVG/Report artifacts to verify the
parity scatter plot and ML Models table. The Runtime Summary tab also shows
train/test counts, RMSE/MAE/R2, leakage status, coefficient summary, and loss
history.

## Completion Scope

Current completion sources:

```text
- language keywords
- built-in quantity kinds from eng_compiler
- built-in units from eng_compiler
- starter snippets for script, CSV schema, and simple thermal system blocks
- uncertainty and ML user-test snippets
```

This is a tester IDE completion surface, not a full LSP yet. It is enough for
release users to explore current language examples and quickly produce new
small scripts without remembering every quantity or unit spelling.

## VS Code Extension Preview

The release zip also contains a VS Code extension preview:

```text
tools/englang-vscode-preview-<version>.vsix
```

This extension shares the compiler-facing diagnostic/hover/completion shape,
but it is secondary for v1.0.3. The primary no-install user test path is
`eng-ide.exe`.
