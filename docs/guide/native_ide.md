# Tauri Tester IDE

The public preview ships `eng-ide.exe` as a portable Tauri/WebView tester IDE.
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
metadata is available, and the official domain/component track example produces
domain, component, and connection metadata.

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
  Compact Run, Check, Save, Report, and Plot actions, diagnostic counts, and
  current status.

Workspace bar
  Shows the resolved workspace root and current file path. Files inside the
  workspace open from the Explorer; outside `.eng` files can also be dropped
  onto the window for editing and running.

Left Explorer
  Dense workspace browser for `examples/`, `stdlib/`, and `docs/`. Files open
  into editor tabs.

Editor
  Multi-file tabs support switching and closing files. The editor keeps dirty
  state per tab and shows completion suggestions near the caret from current
  symbols, keywords, snippets, quantity kinds, and units. Press `Ctrl+Space` to
  force suggestions, `Tab` or `Enter` to insert, and `Esc` to dismiss.

Right Sidebar
  Variables, Plot, and Run tabs. After a successful run, source symbols,
  runtime variables, and Args values are summarized in a table. Clicking a
  variable row expands canonical unit, dimension, role, and line metadata. Plot
  previews live beside Variables so the bottom terminal keeps a stable height.

Bottom Panel
  Problems and Terminal tabs. The Terminal uses an EngLang prompt, supports
  `clear`, `reset`, `check`, `run`, and one-line top-level commands. Terminal
  history is append-only during normal use, so diagnostics and prior output do
  not disappear when the next command runs.

Reports and artifacts remain runtime objects by default. The Report toolbar
button and Plot tab artifact button save/open those artifacts on demand after a
successful run. This keeps the IDE focused on code, terminal output, variables,
diagnostics, and plot preview instead of exposing an artifact browser by
default.

## Recommended Smoke Files

Recommended release user-test example:

```text
examples/official/03_integrated_hvac/main.eng
```

See [Integrated HVAC user test](../tutorials/06_integrated_hvac.md) for the
step-by-step flow.

Recommended uncertainty-track smoke:

```text
examples/official/04_uncertainty_core/main.eng
```

Recommended data-driven modeling track smoke:

```text
examples/official/05_data_driven_modeling/main.eng
examples/official/05_data_driven_modeling/residuals.eng
```

Recommended domain/component track smoke:

```text
examples/official/06_domain_port/main.eng
```

## VS Code Extension Preview

The release zip also contains a VS Code extension preview:

```text
tools/englang-vscode-preview-<version>.vsix
```

The extension shares the compiler-facing diagnostic/hover/completion shape, but
it is secondary for the current public preview. The primary no-install user test
path is `eng-ide.exe`.
