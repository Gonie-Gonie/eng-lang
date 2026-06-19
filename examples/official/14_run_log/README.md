# Official 14 - Run Log

This example exercises the structured runtime message seed that is now part of
the supported side-effect surface.

It demonstrates:

- `print "..."` for direct CLI/debug output
- `log info "..."`, `log warn "..."`, `log debug "..."`, and `log error "..."`
- unit-aware interpolation inside log messages
- `run_log.json` generation for tooling and IDE inspection

`print` remains lightweight human output. `log <level>` is structured runtime
message metadata.
