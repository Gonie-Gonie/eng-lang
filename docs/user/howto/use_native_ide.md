# Native Tester IDE

The release ships `eng-ide.exe` as a portable Tauri/WebView engineering review
cockpit. Rust remains the authoritative compiler/runtime backend, while the
frontend is static HTML/CSS/JS embedded into the executable.

The primary IDE experience is inspection of engineering meaning:

- variables
- quantities and units
- schemas and data-boundary failures
- TimeSeries axes, ranges, statistics, and plots
- metrics and validation results
- report/review artifacts
- provenance and side-effect artifacts

Solver, component graph, residual, and dependency panels are advanced
inspection panels. They support implementation and debugging, but they are not
the first IDE story.

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
metadata is available, and core runtime artifacts can be inspected. It also
covers advanced solver/component inspector data for regression coverage.

The target user PC does not need Node, npm, Python, Rust, or Visual Studio Build
Tools after the portable package has been downloaded. On Windows, the GUI uses
the system WebView2 runtime.

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
  symbols, keywords, snippets, quantity kinds, units, and stdlib workflow
  module surfaces such as `eng.path`, `eng.io`, `eng.fs`, and `eng.process`.
  The base completion vocabulary comes from the same LSP editor metadata used
  by the VS Code extension, with only larger native IDE snippets added on top.
  Checked files use compiler-backed semantic tokens for colored keyword, unit,
  quantity, workflow, and review-risk highlighting. If the buffer has changed
  since the last check, the overlay falls back to metadata-backed lexical
  coloring until Check or Run refreshes token ranges. The fallback vocabulary
  comes from the same LSP editor syntax catalog used by the VS Code extension.
  Tab and Shift+Tab indent or outdent the current line or selected block,
  Ctrl+/ toggles `#` line comments, and Enter preserves block indentation. The
  editor meta bar shows the current caret line, column, highlight category, and
  quantity/unit detail when the caret is on a checked token.

Right Sidebar
  Vars, Checks, Schema, Time, Tables, Reads, Plot, Review, Quality, Effects,
  Artifacts, and Run tabs are the primary review path. After a successful run,
  source symbols, runtime variables, Args values, schema summaries, unit
  conversions, TimeSeries ranges/statistics, metrics, validations, uncertainty
  summaries/propagation metadata, time alignments, artifact paths, and JSON
  artifact outlines are summarized in tables. The Highlight tab shows highlight
  categories, token counts, and source ranges from the current check. The Net
  tab summarizes network boundaries, network events, cache events, hashes, and
  cache keys for workflows that use `eng.net` or `eng.cache`. Uncertain scalar
  bindings also appear in the variable view with their representation and
  summary values.

Advanced panels
  Assembly, dependency, residual, component graph, system, and solver result
  panels expose implementation evidence for scoped simulation paths. They
  should be read as review artifacts and regression coverage, not as proof of a
  broad solver platform.

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
plot manifest outlines inspectable, including `output_manifest.json`
`artifact_registry` sections.

## Recommended Smoke Files

Recommended first user-test examples:

```text
examples/official/01_csv_plot/main.eng
examples/official/09_command_where_with/main.eng
examples/official/16_test_assert_golden/main.eng
```

Recommended advanced/internal inspection examples:

```text
examples/internal/17_measured_vs_simulated/main.eng
examples/internal/04_uncertainty_core/main.eng
examples/internal/05_data_driven_modeling/main.eng
examples/internal/06_domain_port/main.eng
```

Solver-heavy smoke paths under `examples/advanced_solver` are useful for
implementation coverage, but they are not first-user walkthroughs.

## Roadmap Order

1. TimeSeries, schema, unit, report, and review panels.
2. Side-effect and provenance panels.
3. Scoped system simulation result inspection.
4. Component graph, residual, dependency, and solver internals.
5. Editor parity and long-running editor integration.

## VS Code Extension

The release zip also contains a VS Code extension:

```text
tools/englang-vscode-<version>.vsix
```

From a source checkout, build and install the local VSIX with:

```bat
.\dev.bat vscode-install
```

Use `.\dev.bat vscode-package` to only write
`dist\local-vscode\tools\englang-vscode-<version>.vsix` and install it through
VS Code's `Extensions: Install from VSIX...` command.

The VS Code extension defaults to quieter file checks for the Problems panel.
Set `"englang.problemsSource": "live"` to update Problems while typing from the
current unsaved buffer.

The extension shares the same compiler-backed diagnostics, hover, completion,
and semantic highlighting data as the native IDE. It is useful when you prefer
VS Code, while `eng-ide.exe` remains the primary no-install review path for the
current release.
