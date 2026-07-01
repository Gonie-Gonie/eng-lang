# EngLang VS Code Extension Preview

This extension is the secondary IDE preview for user testing. It intentionally uses
the shipped `eng.exe` command instead of embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting for workflow keywords,
  schema/types, units, built-in functions, with-block options, and literals
- diagnostics from `eng ide-check`
- optional diagnostics/completion/hover metadata from `eng-lsp --snapshot`
- debounced unsaved-buffer diagnostics from `eng-lsp --snapshot-stdin`
- semantic highlighting from `eng-lsp --snapshot-stdin`, covering roles such as
  variables, parameters, properties, quantities, units, reports, validations,
  and side-effect/external workflow boundaries
- hover from compiler review metadata
- position-aware completion from `eng-lsp --completion-stdin`
- snippets from `snippets/eng.json`
- quick fixes for `:=` and stale `struct Args` migration diagnostics
- commands to check, run, and open the latest generated report

## Install From Portable Package

1. Extract the EngLang portable zip.
2. In VS Code, run `Extensions: Install from VSIX...`.
3. Select `tools/englang-vscode-<version>.vsix` from the extracted
   EngLang folder.
4. Open the extracted folder or any EngLang project folder.
5. Open a `.eng` file and run `EngLang: Check Current File`.

The packaged VSIX contains `eng.exe` and experimental `eng-lsp.exe`, so no Rust
setup is required for IDE preview diagnostics or LSP smoke checks. The default
diagnostics backend still uses direct `eng.exe` commands. To exercise the
snapshot path, set:

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
backend runs `eng.exe ide-check <file.eng>` on open/save and manual check. The
optional `lsp-snapshot` backend runs `eng-lsp.exe --snapshot <file.eng>` for
experimental diagnostics, hover metadata, and completion metadata, while
run/report commands still use `eng.exe`.

Dirty buffers are checked after a short typing pause with
`eng-lsp.exe --snapshot-stdin <file.eng>`, so Problems can update before the
file is saved. Set `englang.lintOnChange = false` to keep diagnostics limited
to open/save/manual checks.

Semantic highlighting uses the same snapshot-stdin path so unsaved edits receive
role-aware token colors without waiting for a file save. Set
`englang.semanticHighlighting.enabled = false` to fall back to TextMate-only
highlighting.

Completion requests call `eng-lsp.exe --completion-stdin <file.eng> <line>
<character>` with the current unsaved buffer. JavaScript does not maintain a
separate keyword, type, quantity, or unit table.
