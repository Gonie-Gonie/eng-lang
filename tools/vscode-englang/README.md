# EngLang VS Code Extension Preview

This extension is the VS Code preview for EngLang editing and local workflow
checks. It intentionally uses the shipped EngLang executables instead of
embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting for workflow keywords,
  schema/types, units, built-in functions, with-block options, and literals
- stable file diagnostics from the EngLang CLI checker
- optional live editor diagnostics, hover, completion, symbols, and folding from
  the EngLang editor service
- debounced diagnostics for unsaved buffers after a short typing pause
- semantic highlighting for unsaved buffers, covering roles such as
  variables, parameters, properties, built-in workflow helpers, module
  namespaces, quantities, units, reports, validations, and side-effect/external
  workflow boundaries
- packaged semantic token modifier and TextMate fallback scope metadata so
  themes can color EngLang roles consistently
- subtle review-risk line and overview-ruler markers for high and medium risks
- highlight-token inspection command that opens the current token payload as JSON
  for theme/highlighting inspection
- hover from compiler review metadata
- position-aware completion from compiler/editor metadata
- current-file go-to-definition from document symbols
- snippets from `snippets/eng.json`
- quick fixes for `:=`, stale `struct Args`, removable `script` wrapper
  migration diagnostics, ambiguous unit-to-quantity annotations, safe
  missing-unit suffix fixes for unit arithmetic diagnostics, schema column
  annotation migrations, required file-mutation `with` options, invalid
  sampling seed values, and missing repro-profile sampling seeds
- commands to check, run the current file or a bundled example with saved
  artifacts, open a current-file review panel, open the current file review
  JSON, open the latest generated report, and inspect review/run artifacts such
  as `review.json`, `result.engres`, `report_spec.json`,
  `output_manifest.json`, `run_log.json`, `static_run_plan.json`,
  `run_plan.json`, `run_lock.json`, `process_results.json`,
  `cache_manifest.json`, `test_results.json`, `plots/plot_spec.json`,
  `plots/plot_manifest.json`, and `plots/timeseries.svg`
- `EngLang: Switch Execution Profile...` for choosing the `normal`, `safe`, or
  `repro` profile used by `EngLang: Run Current File`

## Install From Portable Package

1. Extract the EngLang portable zip.
2. In VS Code, run `Extensions: Install from VSIX...`.
3. Select `tools/englang-vscode-<version>.vsix` from the extracted
   EngLang folder.
4. Open the extracted folder or any EngLang project folder.
5. Open a `.eng` file and run `EngLang: Check Current File`.

The packaged VSIX contains `eng.exe` and the EngLang editor service, so no Rust
setup is required for diagnostics or editor-service smoke checks. The default
Problems source still uses stable file checks. To try live editor diagnostics,
set:

```text
englang.diagnosticsBackend = lsp-snapshot
```

## Install From Source

Build the CLI first:

```bat
.\dev.bat build
```

Then open this folder as an extension development host, or set:

```text
englang.runtimePath = C:\path\to\eng.exe
englang.lspPath = C:\path\to\eng-lsp.exe
```

## Current Scope

This is not a persistent LSP-client extension yet. The default `eng-cli`
diagnostics source runs stable file checks on open/save and manual check. The
optional `lsp-snapshot` source uses the EngLang editor service for live
Problems data aligned with hover, completion, symbols, and folding, while
run/report/artifact commands still use `eng.exe`.
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
JSON` runs the same current-file review command and opens the normalized JSON
directly, without requiring a prior run. `EngLang: Open Last Run Review JSON`
opens the `build/result/review.json` artifact from the last saved run.
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

Hover uses the current unsaved buffer through the editor-service snapshot, so
quantity, unit, kind, and status details stay aligned with live diagnostics and
semantic highlighting.

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

Review-risk decorations add a subtle left border and overview-ruler mark for
high and medium review risks without changing source text. Set
`englang.reviewRiskDecorations.enabled = false` to hide those markers while
keeping diagnostics and semantic highlighting enabled.

Completion uses the current unsaved buffer and compiler-owned editor metadata.
JavaScript does not maintain a separate keyword, type, quantity, or unit table.
If live completion is unavailable, the extension falls back to the generated
completion seed from `generated/editor/englang-editor-metadata.json`.
`eng-lsp.exe --editor-metadata` exposes that completion seed and the
semantic-token legend used by editor contract checks.

Format Document uses `eng-lsp --format-stdin` on the current unsaved buffer, so
VS Code and the command-line formatter share the compiler-owned formatting
rules. JavaScript does not maintain a separate indentation or block-formatting
implementation.

Go-to-definition asks `eng-lsp --definition-stdin` about the current unsaved
buffer, so static file imports can resolve to their imported source files. If
live definition lookup is unavailable, the extension falls back to document
symbols from the current buffer for top-level symbols and nested symbols such as
schema fields, class fields, component ports, and object members.

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

The extension loads its semantic-token legend from
`generated/editor/englang-editor-metadata.json`, generated from
`eng-lsp --editor-metadata`. The same metadata file also provides the static
completion fallback used when live LSP completion is unavailable. Regenerate it
after LSP completion or semantic legend changes:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat ide-check
```
