# Native Tester IDE

The release ships `eng-ide.exe` as a portable native engineering review
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

`setup` installs the pinned local Rust/MinGW/Python tooling. The native IDE
frontend is static, so setup does not install Node/npm.

## Interface

Top toolbar
  Compact icon-backed Run, Check, Save, Save All, Report, and Plot actions,
  diagnostic counts, execution profile selection for `normal`, `safe`, and
  `repro`, output folder opening, and current status. Editing refreshes
  diagnostics after a short pause; Check forces an immediate refresh.

Workspace bar
  Shows the resolved workspace root, current file path, and Run Dir. Files
  inside the workspace open from the Explorer; outside `.eng` files can also be
  dropped onto the window for editing and running.

Left Explorer
  Dense collapsible workspace browser for `examples/`, `stdlib/`, and `docs/`,
  plus Open Editors. The compiler-backed Outline lists declarations and local
  symbols from the current unsaved buffer; filter by name, kind, or detail and
  select a row to jump to its exact source range. Ctrl+Shift+O focuses the
  Outline filter, and Enter opens its first result. Files open into editor tabs,
  and the active Run Dir is highlighted.

  Ctrl+T opens compiler-backed workspace symbol search. It searches `.eng`
  declarations across the workspace, prefers the current contents of every
  modified open EngLang tab over its saved file, and supports arrow-key
  selection, Enter navigation, and Escape. Opening a result preserves modified
  tabs and selects the compiler's exact UTF-16 source range.

Editor
  Multi-file tabs support switching and closing files. The editor keeps dirty
  state per tab and shows completion suggestions near the caret from current
  symbols, keywords, snippets, quantity kinds, units, and stdlib workflow
  module surfaces such as `eng.path`, `eng.io`, `eng.fs`, and `eng.process`.
  The base completion vocabulary comes from the same generated editor catalog
  used by the VS Code extension, with only larger native IDE snippets added on
  top. Checked files use role-aware colors for keywords, units, quantities,
  workflow operations, and review-risk highlighting. As the buffer changes,
  the editor keeps immediate shared syntax colors and automatically refreshes
  precise colors and Problems diagnostics from the unsaved buffer after a
  short typing pause. Live analysis resolves recursive static imports from
  every other modified open EngLang tab before disk, and discards a result if
  any participating tab changes during the check. Check and Run still force an
  immediate refresh. A fixed line-number gutter stays aligned with the editable
  and highlighted source while scrolling, updates as lines are added or
  removed, and widens automatically for long files.
  Tab and Shift+Tab indent or outdent the current line or selected block,
  Ctrl+S saves the active buffer, and Ctrl+/ toggles `#` line comments.
  Find in the toolbar or Ctrl+F searches the current buffer, preloading a
  single-line selection when available. Enter, Shift+Enter, F3, and Shift+F3
  move through matches with wraparound; Match Case narrows the results and
  Escape closes the search bar. Enter in the editor preserves block
  indentation. F12, Ctrl+click, or Definition in the checked-token meta bar
  jumps to the compiler-resolved definition using the current unsaved buffer
  and every other modified open EngLang tab in the workspace. Definitions in
  the current file, static imports, and bundled stdlib modules open at their
  exact UTF-16 source range; changed imports and declarations are resolved from
  open text before disk, and an already-open dirty target tab is reused without
  reloading it. If any participating tab changes during lookup, navigation is
  cancelled before a file is opened. Shift+F12 or References keeps
  compiler-recognized read/write highlights in the current buffer and lists
  openable workspace locations whose static file-import chain resolves to the
  same declaration. Unrelated same-name symbols, comments, plain strings,
  literals, units, and same-named locals in other function scopes are excluded.
  References pass every modified open EngLang tab in the workspace to the
  compiler, so changed declarations and imports are resolved from open text
  before disk. F2 or Rename uses the same snapshot to prepare the symbol and
  apply verified edits for its static-import identity. Every affected file and
  UTF-16 range is validated before any buffer changes; modified target tabs are
  edited in memory and all affected files remain open and modified until Save
  or Save All. If any participating tab changes during the request, the whole
  result is discarded. Built-ins, members, reserved names, conflicts,
  incomplete semantic coverage, overlapping edits, stale source ranges, and
  cross-file edits outside the current workspace are rejected. The `{}`, `[]`, `()`, and `"`
  auto-close or wrap selections. Typing `}` on an
  indented blank line aligns the brace with its block, and Backspace removes an
  empty pair. Format applies the same compiler-owned formatter used by VS Code
  and keeps the buffer dirty until you save or run. Opening a path that already
  has a tab reuses that tab without reading the file again, so explorer and path
  navigation cannot replace an unsaved buffer. Save, Save All, close-with-save,
  toolbar Run, and terminal `run` compare their targets with the disk content
  last opened or successfully saved. Writes are limited to existing files inside
  the workspace. Run preflights every open workspace tab in one batch, writes
  only changed buffers, and then executes the current file. This keeps dirty
  EngLang imports and opened data files consistent with the source reviewed in
  the editor; unrelated external tabs are not written. If Git, a formatter, or
  another tool changed any Save All or Run target, the full batch stops before
  any file is written and runtime does not start. Every affected tab remains
  modified. Choose Discard when closing a conflicted tab and reopen it to load
  the disk version, then reapply the intended edit. Runtime analysis is applied
  only while the complete saved tab snapshot remains current. The editor meta
  bar shows a clickable file/symbol breadcrumb for the caret's checked scope,
  plus the current line, column, bracket match location, highlight category,
  and quantity/unit detail. Symbol breadcrumbs disappear while analysis is
  stale, so they cannot navigate by an older buffer shape. Closing a
  dirty tab offers Save, Discard, and Cancel. Closing the IDE with dirty tabs
  offers Save All, Discard All, and Cancel. The toolbar Save All action persists
  every modified tab without closing the IDE. Each open tab retains its caret,
  selection, and horizontal/vertical editor position across tab switches and
  inspector rerenders. When the caret is between
  checked tokens, it names the nearest highlight on the same line.
  Checked-token actions can jump directly to related sidebar panels such as
  Schema, Time, Checks, Effects, Network, Model, DB, Units, or Variables.
  Hover titles and the Highlight panel use role and status labels such as
  Model field, DB connection field, or Domain compatible instead of
  internal metadata ids.

Right Sidebar
  Variables, Units, Schema, Time, Tables, Reads, Plot, Review, Highlight,
  Quality, Checks, Effects, Artifacts, and Run tabs are the primary review path. After a successful run,
  source symbols, runtime variables, Args values, schema summaries, unit
  conversions, TimeSeries ranges/statistics, metrics, validations, uncertainty
  summaries/propagation metadata, time alignments, artifact paths, and JSON
  artifact outlines are summarized in tables. The Highlight tab shows whether
  analysis is current, in progress, or unavailable, plus filtered counts,
  color-coded domain coverage, highlight categories, source ranges, and
  per-highlight copy actions from the latest buffer analysis. It also lists
  semantic references requested from the caret, reports reference/file counts,
  and lets each current-file read/write or workspace location open at its exact
  source range. The Network
  tab summarizes network boundaries, network events, cache events, hashes, and
  cache keys for workflows that use `eng.net` or `eng.cache`. Uncertain scalar
  bindings also appear in the variable view with their representation and
  summary values.

Advanced panels
  Modules, Workflow, Objects, Assembly, Kernel, Case, Model, and DB panels expose
  implementation evidence for native workflows and scoped simulation paths. The
  Workflow panel shows run graph counts, process-results status, zero external
  process evidence, and graph hashes after a run. These panels should be read
  as review artifacts and regression coverage, not as proof of a broad solver
  platform.

Bottom Panel
  Problems and Terminal tabs. Problems can be filtered by severity, diagnostic
  code, free text, or line; clicking a row jumps to its source line. F8 and
  Shift+F8, or the Previous/Next arrow controls, cycle through the currently
  filtered diagnostics in source order, wrap at the file boundaries, and select
  the exact checked range. Navigation waits for current-buffer analysis rather
  than using stale ranges. Quick Fix...
  on a row, Quick Fix at cursor, or Ctrl+. requests compiler-provided repairs
  for the exact diagnostic from the current unsaved buffer. A single repair is
  applied immediately; multiple repairs open a choice dialog. The IDE accepts
  only bounded, non-overlapping UTF-16 edits for the current file, rejects stale
  buffers or other-file edits, and leaves the repaired buffer modified until
  Save or Save All. Copy at cursor copies the current or nearest same-line
  diagnostic, the row Copy action copies that diagnostic, and Copy visible
  copies the current filtered list with file, line, column, range, source line,
  severity, code, message, and help text for sharing or issue notes.
  The Terminal uses an EngLang prompt, supports `clear`, `reset`, `check`,
  `run`, and one-line top-level commands. Terminal history is append-only
  during normal use, so diagnostics and prior output do not disappear when the
  next command runs. `cd <dir>` changes the terminal Run Dir without changing
  the open file.

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

Run `.\dev.bat vscode-status` to check the built VSIX path, installed EngLang extension folders, running VS Code processes, Package freshness and Install freshness results, and whether reinstall is currently blocked.
Use `.\dev.bat vscode-package` to only write
`dist\local-vscode\tools\englang-vscode-<version>.vsix` and install it through
VS Code's `Extensions: Install from VSIX...` command. Close all VS Code windows before reinstalling EngLang;
an open VS Code window can keep the existing extension folder locked, and
`.\dev.bat vscode-install` checks for that before starting the release build.
When a built VSIX already exists, the preflight error includes its path for
manual installation after closing or reloading VS Code. The wrapper runs the VS
Code CLI with an ignored temporary user-data directory for CLI logs while
installing into the normal user extension directory.

After installing, run `EngLang: Show Tooling Status` in VS Code to open a
summary-first JSON status view with the extension version, selected check/run
and live editor tool paths, configured-path/source status, diagnostics mode,
saved-file/live Problems diagnostics toggles, and role-aware highlighting setting.

The VS Code extension defaults to quieter file checks for the Problems panel.
Run `EngLang: Switch Diagnostics Mode...` and choose `live`, or set
`"englang.diagnosticsMode": "live"`, to update Problems while typing from the
current unsaved buffer. The mode switch command refreshes the active EngLang
editor immediately; switching back to `file` clears stale live Problems for an
unsaved active buffer until the file is saved. Direct settings changes to
diagnostics mode or Problems settings also refresh or clear the active EngLang editor.

The extension shares the same checked-code diagnostics, hover, completion, and
role-aware highlighting data as the native IDE. Live diagnostics, hover,
completion, go-to-definition, Find All References, rename preparation, and
rename pass the current file plus every modified open EngLang file in the
workspace to the compiler. Recursive static imports use open text before disk,
and results are discarded if the participating document set, dirty state, or
version changes. Problems, role-aware colors, and review decorations in other
open editors refresh after a modified import changes or closes. Saving an open
import also refreshes its open dependents. When Git, a formatter, or another
tool creates, changes, or deletes a closed workspace `.eng` import, VS Code
refreshes those dependents automatically; generated `build`, `target`, and
`dist` trees are ignored. Saved workspace files are added when their static
file-import chain resolves the symbol to the same declaration;
unrelated same-name symbols are excluded. Rename rejects the whole operation
when a participating buffer changes, any affected file has incomplete semantic
coverage, or a conflict is found. Local variables and parameters remain
current-file operations, while built-ins and members are not renameable. The
extension is useful when you prefer VS Code,
while `eng-ide.exe` remains the primary no-install review path for the current
release.
