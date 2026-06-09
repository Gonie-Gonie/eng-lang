# Official 16: Test Assert Golden

This example exercises the `v0.9-preview` test/assert/golden seed.

It demonstrates:

- `test "name" { ... }` blocks
- numeric `assert` checks with optional `within` tolerance
- explicit artifact golden comparison with `golden "artifact" matches file(...)`
- `test_results.json` generation for IDEs, CI, and review tooling
