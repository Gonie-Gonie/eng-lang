# 15 Process Result

This example exercises the external process surface that is now part of the
supported side-effect surface.

It demonstrates:

- `result = run command "..."`
- `with { args = [...] }`
- optional `tool_version`, `expected_outputs`, and `allow_failure` metadata
- `ProcessResult` review metadata
- `process_results.json` generation for tooling and IDE inspection, including
  stdout/stderr hashes

External process execution is intentionally explicit. Non-zero exits fail the
run unless `with { allow_failure = true }` is attached to the process owner.
