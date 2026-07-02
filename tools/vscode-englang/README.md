# EngLang VS Code Extension Preview

This extension is the VS Code preview for EngLang editing and local workflow
checks. It intentionally uses the shipped EngLang executables instead of
embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting for workflow keywords,
  schema/types, units, built-in functions, with-block options, and literals
- stable file diagnostics from the EngLang CLI checker
- optional live editor diagnostics, hover, completion, document/workspace
  symbols, and folding for the current unsaved buffer
- debounced diagnostics for unsaved buffers after a short typing pause
- semantic highlighting for unsaved buffers, covering roles such as
  variables, parameters, properties, built-in workflow helpers, module
  namespaces, quantities, units, reports, validations, and side-effect/external
  workflow boundaries
- packaged semantic token modifier and TextMate fallback scope metadata so
  themes can color EngLang roles consistently
- subtle review-risk line and overview-ruler markers for high and medium risks
- highlight-token inspection command for checking how the current file is
  colored
- hover from compiler review metadata
- position-aware completion from compiler/editor metadata
- current-file go-to-definition from document symbols
- workspace symbol search across `.eng` files in the open workspace
- snippets from `snippets/eng.json`
- quick fixes for `:=`, stale `struct Args`, removable `script` wrapper
  migration diagnostics, ambiguous unit-to-quantity annotations, safe
  missing-unit suffix fixes for unit arithmetic diagnostics, schema column
  annotation migrations, required file-mutation `with` options, invalid
  network retry/timeout/body-size policies, pinned response SHA-256 mismatches,
  sampling seed values, and missing repro-profile sampling seeds
- commands to check, run the current file or a bundled example with saved
  artifacts, open a current-file review panel, open current-file review data,
  open the latest generated report, and inspect last-run review data, result
  data, report data, output lists, run logs, run graphs, reproducibility locks,
  external process results, cache records, test results, plot data, plot output
  lists, and plot SVGs
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
Problems source uses stable file checks. In Settings, switch EngLang diagnostics
to live editor checks to update Problems while typing. In `settings.json`, set:

```json
{
  "englang.problemsSource": "live"
}
```

## Install From Source

To build and install the extension from the current checkout:

```bat
.\dev.bat vscode-install
```

This builds a release `eng.exe` and `eng-lsp.exe`, packages
`dist\local-vscode\tools\englang-vscode-<version>.vsix`, and installs it with
the VS Code `code` CLI. Reload any VS Code windows that already had `.eng`
files open.

To build the VSIX without installing it:

```bat
.\dev.bat vscode-package
```

If the `code` CLI is not on PATH, run `Extensions: Install from VSIX...` in VS
Code and select the generated VSIX. For extension-host development instead of
local installation, open `tools\vscode-englang` in VS Code and launch the
extension development host. If you run directly from source without packaging,
set:

```text
englang.runtimePath = C:\path\to\eng.exe
englang.lspPath = C:\path\to\eng-lsp.exe
```

## Current Scope

The extension is a local editor client for the bundled EngLang tooling. It uses
short-lived editor requests for live Problems, hover, completion, document
symbols, workspace symbols, folding, semantic tokens, definition, formatting,
and quick fixes. This keeps VS Code behavior aligned with the compiler while
the long-running editor protocol continues to evolve. The default Problems
source runs stable file checks on open/save and manual check. Set
`englang.problemsSource` to `live` to update Problems from the current unsaved
buffer while typing. The legacy
`englang.diagnosticsBackend` setting is still accepted for older workspaces,
but new settings should use `englang.problemsSource`.
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
available `build/result` outputs directly. `EngLang: Open Current File Review
Data` runs the same current-file review command and opens the normalized review
data directly, without requiring a prior run. `EngLang: Open Last Run Review
Data` opens the `build/result/review.json` artifact from the last saved run.
`EngLang: Open Last Generated Output...` reads
`build/result/output_manifest.json` and opens any existing file recorded by the
last run, including generated CSV/text outputs and review artifacts that are not
listed as fixed commands.

Dirty buffers are checked after a short typing pause, so Problems can update
before the file is saved. Set `englang.lintOnChange = false` to keep
diagnostics limited to open/save/manual checks.

Quick fixes are available for common syntax migrations, quantity/unit
annotations, schema column annotations, side-effect confirmations, and invalid
network/process options such as retry, timeout, body-size, and allow-failure
values.

Hover is computed from the current unsaved buffer, so quantity, unit, kind, and
status details stay aligned with live diagnostics and semantic highlighting.

Semantic highlighting also works on unsaved edits, so role-aware token colors do
not have to wait for a file save. The extension declares EngLang-specific
semantic token modifiers and TextMate fallback scopes for units,
quantities, axes, time series, validation/report roles, side effects, external
boundaries, inputs, state, built-in workflow helper functions, module
namespaces, model artifacts, DB/cache records, workflow steps, and review risks,
so themes without EngLang-specific rules still receive stable color hints. Set
`englang.semanticHighlighting.enabled = false` to fall back to TextMate-only
highlighting. Maintainer-facing scope and semantic-token mapping rules live in
`docs/internal/editor/token_scopes.md`.
`EngLang: Inspect Highlight Tokens` opens a highlight data view with category
and detail counts plus representative source-text samples.

Review-risk decorations add a subtle left border and overview-ruler mark for
high and medium review risks without changing source text. Set
`englang.reviewRiskDecorations.enabled = false` to hide those markers while
keeping diagnostics and semantic highlighting enabled.

Completion uses the current unsaved buffer and compiler-owned editor metadata.
JavaScript does not maintain a separate keyword, type, quantity, or unit table.
If live completion is unavailable, the extension falls back to the generated
completion catalog from `generated/editor/englang-editor-metadata.json`. The
same generated metadata also supplies the semantic highlighting catalog used by
editor contract checks.

Format Document uses the current unsaved buffer, so VS Code and the command-line
formatter share the compiler-owned formatting rules. JavaScript does not
maintain a separate indentation or block-formatting implementation.

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
.\dev.bat vscode-build-grammar
.\dev.bat vscode-grammar-test
```

The grammar smoke writes token-check output under
`build\editor-tests\textmate_tokens\grammar_smoke.json`.

## Editor Metadata

The extension loads its semantic-token legend through `editorMetadata.js` from
`generated/editor/englang-editor-metadata.json`, generated from
`eng-lsp --editor-metadata`. The same metadata file also provides the static
completion fallback used when live LSP completion is unavailable. Regenerate it
after LSP completion or semantic legend changes:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat ide-check
```
