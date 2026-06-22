# Tauri Tester IDE

The release ships `eng-ide.exe` as a portable Tauri/WebView tester IDE.
Rust remains the authoritative compiler/runtime backend, while the frontend is
static HTML/CSS/JS embedded into the executable.

The target user PC does not need Node, npm, Python, Rust, or Visual Studio Build
Tools after the portable package has been downloaded. On Windows, the GUI uses
the system WebView2 runtime.

## Portable User Flow

Download and extract the EngLang portable zip, then run:

```bat
eng-ide.exe
```

For a non-GUI smoke check:

```bat
eng-ide.exe --smoke
```

The smoke path checks that examples are discoverable, compiler completion
metadata is available, the official domain/component track example produces
domain, component, connection, and assembly metadata, and the measured-vs-simulated
workflow produces IDE inspector payloads for schema, TimeSeries, metric,
validation, time alignment, solver result trajectories, and artifact outlines.

## Development Flow

From the repository root:

```bat
.\dev.bat setup
.\dev.bat ide-check
.\dev.bat ide
```

`setup` installs the pinned local Rust/MinGW/Python tooling. The Tauri IDE
frontend is static, so setup does not install Node/npm.

## Interface

Top toolbar
  Compact icon-backed Run, Check, Save, Report, and Plot actions, diagnostic
  counts, execution profile selection for `normal`, `safe`, and `repro`, output
  folder opening, and current status.

Workspace bar
  Shows the resolved workspace root, current file path, and Run Dir. Files
  inside the workspace open from the Explorer; outside `.eng` files can also be
  dropped onto the window for editing and running.

Left Explorer
  Dense collapsible workspace browser for `examples/`, `stdlib/`, and `docs/`,
  plus Open Editors. Files open into editor tabs, and the active Run Dir is
  highlighted.

Editor
  Multi-file tabs support switching and closing files. The editor keeps dirty
  state per tab and shows completion suggestions near the caret from current
  symbols, keywords, snippets, quantity kinds, and units. Press `Ctrl+Space` to
  force suggestions, `Tab` or `Enter` to insert, and `Esc` to dismiss.

Right Sidebar
  Vars, Schema, Time, Plot, Checks, Asm, Artifacts, and Run tabs. After a successful
  run, source symbols, runtime variables, Args values, schema summaries, unit
  conversions, TimeSeries ranges/statistics, metrics, validations, time
  alignments, system/solver metadata, solver state/algebraic/input/output
  result summaries with source-equation evidence, system dependency rows,
  component assembly metadata,
  residual dependency rows, artifact paths, and JSON artifact outlines are
  summarized in tables. Clicking a variable row expands canonical
  unit, dimension, role, and line metadata. Plot views live beside Variables
  so the bottom terminal keeps a stable height.

Bottom Panel
  Problems and Terminal tabs. The Terminal uses an EngLang prompt, supports
  `clear`, `reset`, `check`, `run`, and one-line top-level commands. Terminal
  history is append-only during normal use, so diagnostics and prior output do
  not disappear when the next command runs. `cd <dir>` changes the terminal Run
  Dir without changing the open file.

Reports and artifacts remain runtime objects by default. The Report toolbar
button, Output toolbar button, Plot tab artifact button, and artifact-path rows
save/open artifacts on demand after a successful run. This keeps the IDE
focused on code, terminal output, variables, diagnostics, and plot viewing while
still making `review.json`, `report_spec.json`, `output_manifest.json`,
`run_log.json`, `process_results.json`, `test_results.json`, PlotSpec, and
plot manifest outlines inspectable.

## Recommended Smoke Files

Recommended release user-test example:

```text
examples/internal/03_integrated_hvac/main.eng
```

See [Integrated HVAC user test](../tutorials/06_integrated_hvac.md) for the
step-by-step flow.

Recommended uncertainty-track smoke:

```text
examples/internal/04_uncertainty_core/main.eng
```

Recommended data-driven modeling track smoke:

```text
examples/internal/05_data_driven_modeling/main.eng
examples/internal/05_data_driven_modeling/residuals.eng
```

Recommended domain/component track smoke:

```text
examples/internal/06_domain_port/main.eng
```

## VS Code Extension

The release zip also contains a VS Code extension:

```text
tools/englang-vscode-<version>.vsix
```

The extension shares the compiler-facing diagnostic/hover/completion shape, but
it is secondary for the current release. The primary no-install user test
path is `eng-ide.exe`.
