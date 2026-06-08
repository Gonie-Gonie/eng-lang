# Functions And Imports

This preview example shows file imports, pure unit-aware functions, CLI prints,
and explicit summary CSV export.

Run:

```powershell
cargo run -p eng_cli -- run examples/official/07_functions_imports/main.eng
```

Expected behavior:

- `use "thermal.eng"` imports function definitions without importing entry
  points from the library file.
- `heat_loss` type-checks its parameter and return dimensions.
- `Q_wall` is inferred as `HeatRate`.
- `print` and `export summary to csv` format the result in kW.
