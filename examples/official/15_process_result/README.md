# 15 Process Result

This example exercises the external process seed that was introduced before
v1.0.0 and is now part of the supported side-effect surface.

It demonstrates:

- `result = run command "..."`
- `with { args = [...] }`
- `ProcessResult` review metadata
- `process_results.json` generation for tooling and IDE inspection

External process execution is intentionally explicit. Non-zero exits fail the
run unless `with { allow_failure = true }` is attached to the process owner.
