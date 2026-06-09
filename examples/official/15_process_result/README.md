# 15 Process Result

This example exercises the `v0.8-preview` external process seed.

It demonstrates:

- `result = run command "..."`
- `with { args = [...] }`
- `ProcessResult` review metadata
- `process_results.json` generation for tooling and IDE inspection

External process execution is intentionally explicit. Non-zero exits fail the
run unless `with { allow_failure = true }` is attached to the process owner.
