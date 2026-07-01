# Official 13 - File Operations

This example exercises the filesystem mutation surface that is now part of the
supported side-effect surface.

It demonstrates:

- copying a source-relative text file into the generated output tree
- moving an output file with explicit `confirm = true`
- deleting a generated scratch file with explicit `confirm = true`
- deleting a generated directory with explicit `recursive = true` and
  `confirm = true`
- recording file operation metadata in `review.json`
- recording changed outputs in `output_manifest.json`

All mutation targets are constrained under `build/result` in the current
runtime.
