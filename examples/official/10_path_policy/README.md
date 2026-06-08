# Official 10: Path Policy

This mini example demonstrates the first general-programming policy slice:
typed path arguments, pure path helpers, and provenance-visible filesystem
existence checks.

Run it with:

```powershell
cargo run -p eng_cli -- run examples/official/10_path_policy/main.eng --save-artifacts
```

The review, result, and report spec artifacts record `environment_dependencies`
for the `exists args.input` check.
