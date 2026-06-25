# eng doc Extract Plan

Status: design skeleton.

eng doc extract should statically parse and semantically inspect EngLang source
without executing top-level workflows.

Planned outputs:

- build/docs/symbol_index.json
- build/docs/diagnostics_index.json
- build/docs/artifact_schema_index.json
- build/docs/examples_index.json

The extractor should read doc comments, declarations, signatures, types, units,
quantity kinds, side-effect metadata, diagnostics, and example links. OODocs may
consume these JSON files later to build reference bundles.
