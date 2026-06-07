# EngLang VS Code Extension Preview

This extension is the v1.0.2 IDE preview for user testing. It intentionally uses
the shipped `eng.exe` command instead of embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting
- diagnostics from `eng ide-check`
- hover from compiler review metadata
- quantity, unit, keyword, and snippet completion
- commands to check, run, and open the latest generated report

## Install From Portable Package

1. Extract the EngLang portable zip.
2. In VS Code, run `Extensions: Install from VSIX...`.
3. Select `tools/englang-vscode-preview-<version>.vsix` from the extracted
   EngLang folder.
4. Open the extracted folder or any EngLang project folder.
5. Open a `.eng` file and run `EngLang: Check Current File`.

The packaged VSIX contains `eng.exe`, so no Rust setup is required for IDE
preview diagnostics.

## Install From Source

Build the CLI first:

```bat
.\dev.bat build
```

Then open this folder as an extension development host, or set:

```text
englang.runtimePath = C:\path\to\eng.exe
```

## Current Scope

This is not a full LSP server yet. Diagnostics are refreshed on open/save and
manual check. Unsaved buffer diagnostics are intentionally conservative because
schema and CSV paths are resolved relative to real files.
