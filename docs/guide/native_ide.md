# Native Tester IDE

The v1.0.3 hardening path upgrades the portable native tester IDE. It is built as
`eng-ide.exe` from the Rust workspace and is shipped inside the portable Windows
zip beside `eng.exe`.

The goal is practical user testing before experimental v1.1+ features are
promoted into a public release contract:

```text
- browse official examples, stdlib, and tutorial sources
- create new .eng files
- edit source in a native GUI
- see syntax highlighting and line-level diagnostic tinting
- run compiler diagnostics while editing
- inspect quantity/unit symbol metadata
- inspect v2.0 domain/component declarations and connection status
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

The smoke path checks that the IDE can discover example files, call the same
compiler metadata path used by `eng.exe check`, and read the official v2.0
domain/component example.

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
  status. Settings opens appearance, font, window-size, and layout controls.

Left Explorer
  Opens .eng files from examples/, stdlib/, and selected tutorial sources.
  `examples/official` is shown before compatibility regression, diagnostic,
  and data-quality fixtures. Creates scratch .eng files from a starter
  template. Open File and Open Folder use native OS dialogs.

Main work area
  The center is split as Code on the left and Result on the right. The Result
  pane is separated by a draggable divider and independently scrollable, so
  plots, runtime summaries, and artifact links can be reviewed without leaving
  the editor or covering the right sidebar.

Code
  Native multiline editor with EngLang syntax highlighting and line-level
  diagnostic backgrounds. The editor uses a Windows-friendly monospace stack,
  configurable code font size, and supports vertical scrolling by default.
  Long source lines can be soft-wrapped or left to horizontal scrolling from
  Settings.

Result
  Run Preview renders PlotSpec points inside the IDE with axes, grid lines,
  ticks, and plot-specific rendering for line, scatter, bar, and histogram
  plots. Histogram previews use PlotSpec bin edges when available. Runtime
  Summary and Artifacts are shown below the plot in the same scrollable result
  panel.

Right Sidebar
  Tabbed Inspector, Completions, and Runtime Summary surface. Inspector shows
  variables with quantity kind, display/canonical unit, dimension, source,
  expression, and unit derivation path. The Inspector also includes a Domain
  Graph section for v2.0 files, showing domain variables, conservation
  metadata, package/version metadata, generic domain parameters, component port
  arguments, port resolution status, and connection domain compatibility. It
  also shows schema columns, constraints, missing policies, and CSV promotion
  summaries. After Run, the Runtime tab shows result status, uncertainty
  summaries, ML metrics,
  coefficients, loss history, policy count, system count, and the experimental
  `eng-kernel-plan-v1` kernel plan for the current file, including estimated
  rows, input/output counts, operation-class count, scan count, and complexity
  label. Kernel plan data is planning metadata only; execution still uses the
  normal runtime path.

Bottom panel
  Problems, Output, and Artifacts tabs.
```

## User Settings

The toolbar `Settings` button controls the tester IDE without editing config
files by hand:

```text
- Light or Dark theme
- Comfortable or Compact density
- UI, button, heading, and code font sizes
- long-line soft wrap for the code editor
- Explorer, right Sidebar, Result pane, and bottom panel default sizes
- 1366x768, 1600x920, and 1920x1080 window presets
- custom window width/height
```

Settings are applied immediately and saved for the current portable copy at:

```text
build/ide/settings.json
```

That file is intentionally under `build/` so repository users do not commit
personal UI preferences, while an extracted portable package can still remember
the tester's preferred theme, font scale, and layout.

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
examples/official/05_data_driven_modeling/residuals.eng
```

Run it and inspect the Run Preview/Plot SVG/Report artifacts to verify the
parity scatter plot, residual bar plot, and ML Models table. The Runtime
Summary tab also shows train/test counts, RMSE/MAE/R2, leakage status,
coefficient summary, and loss history.

Recommended v1.4 JIT planning smoke:

```text
examples/official/01_csv_plot/main.eng
```

Check or run it and inspect the Runtime Summary Kernel Plan section. It should
show TimeSeries arithmetic, integration, and statistics-fusion candidates with
`backend = interpreter-fallback`, row estimates from the CSV source, and
operation-class counts. This is not a speedup claim or native codegen path.

Recommended v2.0 domain/component smoke:

```text
examples/official/06_domain_port/main.eng
```

Open it and inspect the right Sidebar > Inspector > Domain Graph section. It
should show Thermal, `Fluid[Medium M]`, and
`MechanicalNode[Frame F, Axis DOF]` domains, package/version metadata, five
components, `Fluid[Water]` and `MechanicalNode[World, X]` ports, and three
`domain_compatible` connections.

## Completion Scope

Current completion sources:

```text
- variables and source identifiers from the current file
- language keywords
- built-in quantity kinds from eng_compiler
- built-in units from eng_compiler
- starter snippets for script, CSV schema, simple thermal system, and
  domain/component port blocks
- uncertainty and ML user-test snippets
```

While editing, the IDE updates the completion filter from the cursor prefix and
shows the first suggestion below the code pane. Press `Tab` to apply that
suggestion. The Completions sidebar mirrors the same ordered list and still
supports mouse insertion. The editor also auto-closes `()`, `[]`, `{}`, single
quotes, and double quotes.

This is a tester IDE completion surface, not a full LSP yet. It is enough for
release users to explore current language examples and quickly produce new
small scripts without remembering every variable, quantity, or unit spelling.

## VS Code Extension Preview

The release zip also contains a VS Code extension preview:

```text
tools/englang-vscode-preview-<version>.vsix
```

This extension shares the compiler-facing diagnostic/hover/completion shape,
but it is secondary for v1.0.3. The primary no-install user test path is
`eng-ide.exe`.
