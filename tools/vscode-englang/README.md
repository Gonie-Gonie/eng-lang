# EngLang VS Code Extension Preview

This extension is the secondary IDE preview for user testing. It intentionally uses
the shipped `eng.exe` command instead of embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting for workflow keywords,
  schema/types, units, built-in functions, with-block options, and literals
- stable file diagnostics from the EngLang CLI checker
- optional editor-service diagnostics/completion/hover metadata from
  `eng-lsp --snapshot`
- debounced unsaved-buffer diagnostics from `eng-lsp --snapshot-stdin`
- semantic highlighting from `eng-lsp --snapshot-stdin`, covering roles such as
  variables, parameters, properties, built-in workflow helpers, module
  namespaces, quantities, units, reports, validations, and side-effect/external
  workflow boundaries
- packaged semantic token modifier and TextMate fallback scope metadata so
  themes can color EngLang roles consistently
- semantic token debug command that opens the current `eng-lsp --snapshot-stdin`
  token payload as JSON for theme/highlighting inspection
- hover from compiler review metadata
- position-aware completion from `eng-lsp --completion-stdin`
- snippets from `snippets/eng.json`
- quick fixes for `:=`, stale `struct Args`, removable `script` wrapper
  migration diagnostics, ambiguous unit-to-quantity annotations, safe
  missing-unit suffix fixes for unit arithmetic diagnostics, and schema column
  annotation migrations
- commands to check, run the current file or a bundled example with saved
  artifacts, open a current-file review panel, open the current file review
  JSON, open the latest generated report, and inspect review/run artifacts such
  as `review.json`, `output_manifest.json`, `run_log.json`, `run_plan.json`,
  `process_results.json`, and `cache_manifest.json`
- `EngLang: Switch Execution Profile...` for choosing the `normal`, `safe`, or
  `repro` profile used by `EngLang: Run Current File`

## Install From Portable Package

1. Extract the EngLang portable zip.
2. In VS Code, run `Extensions: Install from VSIX...`.
3. Select `tools/englang-vscode-<version>.vsix` from the extracted
   EngLang folder.
4. Open the extracted folder or any EngLang project folder.
5. Open a `.eng` file and run `EngLang: Check Current File`.

The packaged VSIX contains `eng.exe` and experimental `eng-lsp.exe`, so no Rust
setup is required for IDE preview diagnostics or editor-service smoke checks.
The default diagnostics source still uses the stable CLI checker. To try the
editor-service snapshot path, set:

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
diagnostics source runs the stable CLI checker on open/save and manual check.
The optional `lsp-snapshot` source runs `eng-lsp.exe --snapshot <file.eng>` for
experimental editor-service diagnostics, hover metadata, and completion
metadata, while run/report/artifact commands still use `eng.exe`.
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

Dirty buffers are checked after a short typing pause with
`eng-lsp.exe --snapshot-stdin <file.eng>`, so Problems can update before the
file is saved. Set `englang.lintOnChange = false` to keep diagnostics limited
to open/save/manual checks.

Semantic highlighting uses the same snapshot-stdin path so unsaved edits receive
role-aware token colors without waiting for a file save. The extension declares
EngLang-specific semantic token modifiers and TextMate fallback scopes for units,
quantities, axes, time series, validation/report roles, side effects, external
boundaries, inputs, state, built-in workflow helper functions, module
namespaces, model artifacts, DB/cache records, workflow steps, and review risks,
so themes without EngLang-specific rules still receive stable color hints. Set
`englang.semanticHighlighting.enabled = false` to fall back to TextMate-only
highlighting.

Completion requests call `eng-lsp.exe --completion-stdin <file.eng> <line>
<character>` with the current unsaved buffer. JavaScript does not maintain a
separate keyword, type, quantity, or unit table.

## Grammar Maintenance

The generated TextMate grammar lives at `syntaxes/eng.tmLanguage.json`. Edit
`syntaxes/eng.tmLanguage.source.json`, then run:

```bat
.\dev.bat vscode-build-grammar
.\dev.bat vscode-grammar-test
```

The grammar smoke writes token-check output under
`build\editor-tests\textmate_tokens\grammar_smoke.json`.
