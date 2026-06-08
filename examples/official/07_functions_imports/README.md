# Functions And Imports

This preview example shows static file imports, importable `const` values,
function-local bindings, top-level execution, CLI prints, and explicit summary
CSV export.

Run:

```powershell
cargo run -p eng_cli -- run examples/official/07_functions_imports/main.eng
```

Expected behavior:

- `use "thermal.eng"` imports `const` and `fn` declarations without importing
  executable top-level bindings.
- the root file runs directly as a top-level workflow.
- `heat_loss` type-checks its parameter and return dimensions.
- `UA_local` and `dT_local` are function locals, not importable symbols.
- `Q_wall` is inferred as `HeatRate`.
- `print` and `export summary to csv` format the result in kW.
