# EngLang VS Code Extension

This extension provides VS Code support for EngLang editing and local workflow
checks. It intentionally uses the shipped EngLang executables instead of
embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting for workflow keywords,
  schema/types, units, built-in functions, with-block options, and literals
- stable file diagnostics from the EngLang CLI checker, with code links and
  legacy/deprecated tags in VS Code Problems
- optional live editor diagnostics, hover, completion, document/workspace
  symbols, and folding for the current unsaved buffer
- debounced diagnostics for unsaved buffers after a short typing pause
- role-aware highlighting for unsaved buffers, covering roles such as
  variables, parameters, properties, built-in workflow helpers, module
  namespaces, quantities, units, reports, validations, and side-effect/external
  workflow boundaries
- packaged role-coloring metadata so themes can color EngLang code
  consistently without custom rules
- optional `EngLang Dark` and `EngLang Light` color themes with explicit
  semantic colors for every EngLang semantic selector contributed by the
  extension
- subtle review-risk line and overview-ruler markers for high and medium risks
- highlight-token inspection command for checking how the current file is
  colored
- hover from compiler review metadata
- position-aware completion from compiler/editor metadata
- current-file go-to-definition from document symbols
- workspace symbol search across `.eng` files in the open workspace
- compiler-owned snippets from generated editor metadata, plus non-overlapping static snippets from `snippets/eng.json`
- quick fixes for `:=`, boolean `==`, stale `struct Args`, removable `script` wrapper
  migration diagnostics, ambiguous unit-to-quantity annotations, safe
  missing-unit suffix fixes for unit arithmetic diagnostics, unterminated/empty string
  interpolation closures, unresolved interpolation literal conversions, interpolation display-unit removals, command target
  parenthesizing, unknown stdlib module replacements, planned/internal stdlib
  import removal, schema column annotation migrations, required file-mutation
  `with` options, invalid network URL/body-method/retry/timeout/body-size policies, invalid
  legacy network `fixture` aliases, legacy response `.hash` aliases,
  raw `read json` field-access promotion edits,
  HeatRate TimeSeries sum-to-integrate repairs,
  process binding conflicts and command/env/cwd values, pinned
  response SHA-256 mismatches, sampling count/seed values, missing repro-profile
  sampling seeds, simulation/solver option repairs, model option fallback
  repairs for invalid test splits, seeds, hidden layers, and epochs,
  unsupported regression algorithm repairs, legacy `select_first_row` migration edits,
  uncertainty constructor argument repairs, direct uncertainty comparison repairs,
  uncertainty propagation option/seed repairs, uncertainty source
  definition/conversion repairs, and golden expected path wrappers. Live
  editor quick fixes are shown first
  and merged with local fallback repairs so partial live editor responses do
  not hide available fixes.
- quick fixes for simple same-block `where` local ordering mistakes where a
  later local definition can be moved before its first use
- quick fixes for simple escaped `where` locals where a reused local binding
  can be promoted to a top-level binding
- commands to check, run the current file or a bundled example with saved
  artifacts, open a current-file review panel, open current-file review data,
  open the latest generated report, and inspect last-run review data, result
  data, report data, generated output lists, run logs, run graphs, reproducibility locks,
  process results, cache records, test results, plot data, plot output
  lists, and plot SVGs
- `EngLang: Switch Diagnostics Mode...` for choosing quieter saved-file checks
  or live unsaved-buffer checks from the Command Palette
- `EngLang: Show Tooling Status` for inspecting a summary-first JSON status view
  with the active check/run and live editor tool paths, fallback/source labels,
  diagnostics mode, Problems diagnostics toggles, role-aware highlighting, and extension version
- `EngLang: Switch Execution Profile...` for choosing the `normal`, `safe`, or
  `repro` profile used by `EngLang: Run Current File`

## Install From Portable Package

1. Extract the EngLang portable zip.
2. In VS Code, run `Extensions: Install from VSIX...`.
3. Select `tools/englang-vscode-<version>.vsix` from the extracted
   EngLang folder.
4. Open the extracted folder or any EngLang project folder.
5. Open a `.eng` file and run `EngLang: Check Current File`.

The packaged VSIX contains the EngLang command-line and editor tooling, so no
Rust setup is required for diagnostics or live editor checks. The default
diagnostics mode uses stable file checks. In Settings, switch EngLang diagnostics
to live editor checks, or run `EngLang: Switch Diagnostics Mode...`, to update
Problems while typing. In `settings.json`, set:

```json
{
  "englang.diagnosticsMode": "live"
}
```

## Install From Source

To build and install the extension from the current checkout:

```bat
.\dev.bat vscode-install
```

This builds a release `eng.exe` and `eng-lsp.exe`, packages
`dist\local-vscode\tools\englang-vscode-<version>.vsix`, and installs it with
the VS Code `code` CLI. Close all VS Code windows before reinstalling EngLang;
VS Code can lock the existing extension folder while it is running, so
`vscode-install` checks for that before starting the release build. The wrapper
runs the CLI from an ignored temporary directory so VS Code log files do not dirty
the checkout. Reload VS Code after installation. The VSIX remains available at
the generated `dist\local-vscode\tools` path.

To build the VSIX without installing it:

```bat
.\dev.bat vscode-package
```

If the `code` CLI is not on PATH, run `Extensions: Install from VSIX...` in VS
Code and select the generated VSIX. For extension-host development instead of
local installation, open `tools\vscode-englang` in VS Code and launch the
extension development host. After installing, run `EngLang: Show Tooling
Status` to confirm the summary, bundled check/run tool and live editor tool
paths, fallback/source labels, the current diagnostics mode, and per-feature live
editor routing. If you run directly from source without packaging,
set:

```text
englang.runtimePath = C:\path\to\eng.exe
englang.lspPath = C:\path\to\eng-lsp.exe
```

## Current Scope

The extension is a local editor client for the bundled EngLang tooling. It uses
on-demand live editor checks for live Problems, hover, completion, document
symbols, workspace symbols, folding, role-aware highlight data, definition, formatting,
and quick fixes. This keeps VS Code behavior aligned with the compiler while
the long-running editor protocol continues to evolve. The default diagnostics
mode runs stable file checks on open/save and manual check. Set
`englang.diagnosticsMode` to `live` to update Problems from the current unsaved
buffer while typing, or run `EngLang: Switch Diagnostics Mode...` and choose
`live`. If an older workspace already has `englang.problemsSource` or
`englang.diagnosticsBackend`, the extension still accepts it as a compatibility
alias. New workspaces should use `englang.diagnosticsMode`.
`EngLang: Run Current File`
passes `--profile <englang.executionProfile> --save-artifacts`, so the
generated `build/result` review artifacts are available to the open-artifact
commands immediately after a successful run. Use `EngLang: Switch Execution
Profile...` to choose `normal`, `safe`, or `repro` for the workspace.
`EngLang: Run Example...` lists `examples/official/**/main.eng` and
`examples/workflows/**/main.eng`, opens the selected example, then runs it
through the same profile/artifact path.
`EngLang: Open Current File Review Panel` runs
`eng.exe review <file.eng> --json` and opens a VS Code-native summary of
inputs, symbols, schemas, units/quantities, time axes, derived values,
diagnostics, external boundaries, side effects, table transforms, calculations,
validations, caches, risks, and workflow modules. Line cells in the panel jump
back to the matching source line, and the Last Run Artifacts section opens
available `build/result` outputs directly, with the same availability labels as
the artifact picker. `EngLang: Open Current File Review Data` runs the same
current-file review command and opens the normalized review data directly,
without requiring a prior run. `EngLang: Open Last Run Review Data` opens the
`build/result/review.json` artifact from the last saved run.
`EngLang: Open Last Generated Output...` reads
`build/result/output_manifest.json` and opens any existing file recorded by the
last run, including generated CSV/text outputs and review artifacts that are not
listed as fixed commands.

When the diagnostics mode is `live`, dirty buffers are checked after a short
typing pause, so Problems can update before the file is saved. The EngLang output
panel records whether Problems came from file diagnostics or live-buffer
diagnostics and which tool path was selected. Saved-file open/save diagnostics
are controlled by `englang.lintOnSave`; live typing diagnostics are controlled by
`englang.lintOnChange`. If diagnostics cannot parse editor JSON, run
`EngLang: Show Tooling Status` to inspect the selected paths. Set
`englang.lintOnChange = false` to disable live typing checks while keeping
live open/save analysis.

Quick fixes are available for common syntax migrations, quantity/unit
annotations, schema column annotations, side-effect confirmations, and invalid
network/process/sampling options such as retry, timeout, body-size, duplicate
process bindings, process command/env/cwd, allow-failure, sample count, sample
seed values, deterministic cache keys, cache directories, cache TTL values,
model test splits, model seeds, hidden-layer lists, model epochs, and common
simulation/solver option values such as timestep, duration, tolerance, solver,
max-iteration, and initial values.
Simple same-block `where` local ordering diagnostics can move the later
definition before its first use.
Uncertainty diagnostics can also repair common constructor mistakes such as
unsupported distribution kind, unsupported propagation method, invalid sample
count, missing constructor arguments, unknown sources, missing source arguments,
and deterministic sources that should be `measured(...)`. Propagation `with`
blocks can repair invalid uncertainty policy, sample-count, and seed option
values, and can insert a reproducible seed for Monte Carlo propagation. The
provider only answers Quick Fix requests, so refactor and source-action menus
stay scoped to their own providers.

Hover is computed from the current unsaved buffer, so quantity, unit, kind, and
status details stay aligned with live diagnostics and role-aware highlighting.

Role-aware highlighting also works on unsaved edits, so token colors do
not have to wait for a file save. Sample-table member completions include
runtime metadata such as `sample_count`, `row_hash_count`, and `row_preview`.
The extension declares EngLang-specific
role categories and theme fallback hints for units, quantities, axes, time
series, validation/report roles, side effects, external boundaries, inputs,
state, built-in workflow helper functions, solver and uncertainty policy
literals, module namespaces, model artifacts, DB/cache records, workflow steps,
string interpolation variables/properties, format precision, format units,
and review risks, so themes without EngLang-specific rules still receive stable
color hints. For stronger role separation, choose `EngLang Dark` or `EngLang
Light` from VS Code's Color Theme picker; the bundled themes define direct
colors for every EngLang semantic selector contributed by the extension. Set
`englang.semanticHighlighting.enabled = false` to fall back to TextMate-only
highlighting; changing this setting refreshes the current editor colors and
planned/internal symbol markers immediately. Maintainer-facing color mapping
rules live in `docs/internal/editor/token_scopes.md`.
`EngLang: Inspect Highlight Tokens` opens a highlight data view with a plain
status summary, legend, selector/type/detail counts, representative source-text
samples, normalized highlight rows with primary selector, mapped/missing
fallback status, direct selector coverage, theme fallback scopes, and raw
highlight payload for debugging theme or scope mismatches. If no highlight data
is available, the warning can
open
`EngLang: Show Tooling Status` so the selected live editor tool path is visible.
`EngLang: Inspect Highlight Token at Cursor` opens a cursor status summary, the
token under the caret when one exists, the nearest highlight tokens, and the
other highlight tokens on the same line.

Review-risk decorations add a subtle left border and overview-ruler mark for
high and medium review risks without changing source text. Set
`englang.reviewRiskDecorations.enabled = false` to hide those markers while
keeping diagnostics and role-aware highlighting enabled.

Completion uses the current unsaved buffer and compiler-owned editor metadata.
JavaScript does not maintain a separate keyword, type, quantity, or unit table.
If live completion is unavailable, the extension falls back to the generated
completion catalog from `generated/editor/englang-editor-metadata.json`. The
same generated metadata also supplies the highlight legend and syntax catalog
used by editor contract checks. Generic type completions keep their public labels
visible, but insert editable snippets such as `Array[T]` and
`LinearOperator[From -> To]` so type arguments can be replaced immediately.
The generated completion metadata also carries plain insert text and VS Code
snippet insert text for common helper and workflow completions such as
`file(...)`, `read text`, `http get`, and `sample uniform`. Static snippets remain
only for larger examples whose prefixes do not duplicate generated completion
labels.

Format Document and Format Selection use the current unsaved buffer, so VS Code
and the command-line formatter share the compiler-owned formatting rules.
JavaScript does not maintain a separate indentation or block-formatting
implementation.

Go-to-definition uses the current unsaved buffer, so static file imports and
bundled `use eng.<module>` imports can resolve to their source files. If live
definition lookup is unavailable, the extension falls back to document symbols
from the current buffer for top-level symbols and nested symbols such as schema
fields, class fields, component ports, and object members. VS Code's workspace
symbol search scans `.eng` files under each open workspace folder.

## Grammar Maintenance

The generated TextMate grammar lives at `syntaxes/eng.tmLanguage.json`. Edit
`syntaxes/eng.tmLanguage.source.json`, then run:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat vscode-build-grammar
.\dev.bat vscode-grammar-test
```

The source grammar may use `{{...}}` placeholders for compiler-owned keyword,
constant, operator-word, keyword-group, type, unit, option, and highlight-only
legacy unit alias lists. `vscode-build-grammar` expands those placeholders from
`generated/editor/englang-editor-metadata.json`.

The grammar smoke writes token-check output under
`build\editor-tests\textmate_tokens\grammar_smoke.json`.

## Editor Metadata

The extension loads its highlight legend and syntax catalog through
`editorMetadata.js` from `generated/editor/englang-editor-metadata.json`,
generated from `eng-lsp --editor-metadata`. Split generated files are also
written for review: `englang-semantic-legend.json`,
`englang-completions.json`, and `englang-syntax.json`. The same metadata file
provides the static completion fallback used when live completion is
unavailable. New tooling should read the `completion_items` catalog;
`completion_seed` remains a legacy alias for older consumers. The VS Code
extension requires `completion_items` at runtime and validates that
`completion_seed` is only an exact compatibility alias. `syntax_catalog.units`
contains compiler unit labels; `syntax_catalog.legacy_unit_aliases` contains
highlight-only compatibility aliases such as byte-size units and `%`. Regenerate
it after LSP completion, keyword, constant, operator-word, option, type, unit,
legacy unit alias, or highlight legend changes:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat ide-check
```
