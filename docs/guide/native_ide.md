# Native Tester IDE

The v1.0.1 hardening path includes a portable native tester IDE. It is built as
`eng-ide.exe` from the Rust workspace and is shipped inside the portable Windows
zip beside `eng.exe`.

The goal is practical user testing before v1.1 uncertainty work starts:

```text
- open official and local .eng examples
- edit source in a native GUI
- run compiler diagnostics while editing
- inspect quantity/unit hover-style symbol metadata
- use keyword, quantity, unit, and snippet completions
- run the current file and open the generated report
```

This tester IDE is intentionally native Rust GUI code using `eframe`/`egui`.
It does not require a browser, Python, Node, Rust, or Visual Studio Build Tools
on a target user PC after the portable package has been downloaded.

## Portable User Flow

Download and extract the EngLang portable zip, then run:

```bat
eng-ide.bat
```

or:

```bat
eng-ide.exe
```

For a non-GUI smoke check:

```bat
eng-ide.exe --smoke
```

The smoke path checks that the IDE can discover example files and call the same
compiler metadata path used by `eng.exe check`.

## Development Flow

From the repository root:

```bat
.\dev.bat setup
.\dev.bat ide --smoke
.\dev.bat ide
```

`dev.bat ide` runs the native GUI through Cargo. It uses the same pinned
repository-local Rust toolchain as the rest of the project.

## Interface

The native IDE has four main regions:

```text
Top toolbar
  Check, Save, Run, Open Report, entry selection, and current status.

Left examples panel
  Discovers .eng files under examples/ and opens them into the editor.

Center editor
  Native multiline editor with Ctrl+Space completion filtering.

Right intelligence panel
  Completion insertion and compiler-derived symbol metadata.

Bottom diagnostics panel
  Compiler diagnostics and run output.
```

Diagnostics are produced by `eng_compiler::check_source`, so unsaved edits can
be checked without writing temporary source files. Running a program saves the
current file first and then calls the same runtime path as:

```bat
eng.exe run <file.eng> --entry main
```

Generated runtime artifacts are written under:

```text
build/ide-run/
```

## Completion Scope

Current completion sources:

```text
- language keywords
- built-in quantity kinds from eng_compiler
- built-in units from eng_compiler
- starter snippets for script, CSV schema, and simple thermal system blocks
```

This is a tester IDE completion surface, not a full LSP yet. It is enough for
release users to explore current language examples and quickly produce new
small scripts without remembering every quantity or unit spelling.

## VS Code Extension Preview

The release zip also contains a VS Code extension preview:

```text
tools/englang-vscode-preview-<version>.vsix
```

This extension shares the compiler-facing diagnostic/hover/completion shape,
but it is secondary for v1.0.1. The primary no-install user test path is
`eng-ide.exe`.
