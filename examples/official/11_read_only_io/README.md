# Official 11: Read-Only I/O

This mini example demonstrates the v0.4 general-programming slice:
read-only text/json/toml inputs, source-hash provenance, and a multi-source
workflow that combines typed CSV data with auxiliary configuration files.

Run it with:

```powershell
cargo run -p eng_cli -- run examples/official/11_read_only_io/main.eng --save-artifacts
```

The review, result, and report spec artifacts record `environment_dependencies`
for `read text`, `read json`, and `read toml` expressions. These entries include
the resolved source path and source hash.
