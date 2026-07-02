# Diagnostics Model

EngLang diagnostics are action-oriented compiler, runtime, and editor messages.
Each diagnostic has a code, source range, message, and usually a short help
line.

## How Diagnostics Are Surfaced

- `eng.exe check <file.eng>` prints diagnostics for files on disk.
- The VS Code extension and native IDE use `eng-lsp` snapshots for diagnostics
  on saved and unsaved buffers.
- Quick fixes are offered where a repair is local and deterministic, such as
  syntax migrations, missing sample seeds, expected hashes, and known option
  renames.

## Common Diagnostic Areas

- Syntax and migration errors, such as unsupported `script` roots or `:=`.
- Type, quantity, and unit mismatches.
- Schema, table, and config promotion errors.
- Workflow side-effect policy, filesystem mutation, HTTP/cache, DB, model, and
  case diagnostics.

## Related References

- [Diagnostics reference index](../diagnostics/index.md)
- [CLI reference](../cli/index.md)
- [Dimensionless and unit policy](dimensionless.md)
