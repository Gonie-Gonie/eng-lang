# Print And Summary CSV Export

This mini example focuses on top-level execution, `args { ... }`, reusable
`const` values, unit-aware CLI output, and an explicit one-row summary CSV
export.

Run:

```powershell
cargo run -p eng_cli -- run examples/official/08_print_export_summary/main.eng
```

Expected behavior:

- each `print` interpolation is type-checked;
- root `args` values are available as `args.<name>`;
- `const` values can be reused without creating implicit artifacts;
- requested display units must be compatible with each quantity;
- scalar values print with units by default;
- `export summary to csv "summary.csv"` writes `build/result/summary.csv`;
- CSV headers include the requested display units.
