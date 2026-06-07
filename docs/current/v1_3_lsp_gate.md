# v1.3 LSP Gate

This page tracks the first v1.3 LSP path on `main`. The native tester IDE
remains the primary user-test surface. The LSP path is experimental until it is
wired into a release target and manually tested in an editor.

## Current Scope

- `eng-lsp.exe` binary exists through the `eng_lsp` crate.
- `eng-lsp --smoke` checks the official CSV example and prints diagnostic,
  completion, and hover counts.
- `eng-lsp --snapshot <file.eng>` emits `eng-lsp-snapshot-v1` JSON for
  diagnostics, completion items, and hover items.
- `eng-lsp --snapshot-check <file.eng>` verifies that snapshot completion and
  hover data are non-empty without printing the full JSON.
- The portable package includes `eng-lsp.exe` and package smoke runs
  `eng-lsp.exe --smoke`.
- `sensor.`-style member completion uses CSV promotion metadata to return
  schema columns for the promoted table.
- Default `eng-lsp` starts a minimal stdio JSON-RPC loop for:
  - `initialize`
  - `shutdown`
  - `textDocument/didOpen`
  - `textDocument/didChange`
  - `textDocument/didSave`
  - `textDocument/completion`
  - `textDocument/hover`
  - `textDocument/publishDiagnostics`

## Completed On Main

- [x] LSP-shaped diagnostics are derived from compiler diagnostics.
- [x] Completion items include keywords, current typed bindings, schema
  columns, quantity kinds, and units.
- [x] Completion requests at `promoted_table.` return the promoted schema's
  columns instead of the global keyword/unit list.
- [x] Hover items are derived from compiler hover metadata.
- [x] `dev.bat lsp-check` validates smoke and snapshot-check paths.
- [x] `dev.bat ci` runs `lsp-check`.
- [x] Workspace tests include `eng_lsp` snapshot coverage.
- [x] Integration tests spawn `eng-lsp.exe` and round-trip `initialize`,
  `didOpen`, `completion`, `hover`, `shutdown`, and `exit` over stdio.
- [x] `dev.bat package-smoke` validates the packaged `eng-lsp.exe --smoke`
  path.
- [x] `eng-lsp-snapshot-v1` compatibility rules are documented in
  [LSP snapshot reference](../reference/lsp_snapshot.md).
- [x] VS Code extension keeps `eng.exe ide-check` as the default preview
  backend and offers optional `eng-lsp.exe --snapshot` diagnostics metadata via
  `englang.diagnosticsBackend = lsp-snapshot`.

## Remaining Before Support Claim

- [ ] Add editor-level manual tests for diagnostics, completion, and hover in
  VS Code or another LSP client.
- [ ] Add precise diagnostic ranges when compiler spans are ready across all
  diagnostics; current LSP ranges are line-based.

## Verification

```bat
.\dev.bat lsp-check
cargo test -p eng_lsp --test stdio
target\debug\eng-lsp.exe --smoke
target\debug\eng-lsp.exe --snapshot-check examples\official\01_csv_plot\main.eng
```
