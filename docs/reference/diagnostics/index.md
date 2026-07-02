# Diagnostics Reference

This index points to the current diagnostic lookup material. Diagnostics are
owned by the compiler, runtime, and LSP, and are verified through tests and docs
checks.

## Current Lookup Pages

- [Diagnostics model](../language/diagnostics.md)
- [CLI diagnostics and commands](../cli/index.md)
- [Dimensionless and unit policy](../language/dimensionless.md)
- [Side-effect policy](../language/side_effect_policy.md)
- [Report and review artifacts](../artifacts/report_review.md)

## Diagnostic Contract

Each diagnostic should make these points clear:

- what happened
- why it matters for execution, reviewability, or reproducibility
- the smallest valid edit or command to try next

Generated per-code catalog pages can be added here when the extraction pipeline
is ready; this index remains the stable entry point.
