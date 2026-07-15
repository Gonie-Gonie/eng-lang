# Standard Library Reference

The standard library reference is a lookup layer over the machine-readable
module registry. The canonical source for module names, status, artifacts,
diagnostics, examples, and test evidence is
[`stdlib/eng/modules.toml`](../../../stdlib/eng/modules.toml).

For the generated status table and workflow-contract detail, read
[Composite Workflow Base Modules](../../current/workflow_modules.md). That page
is checked by `dev.bat docs-check` against the registry.

## Current Public Lookup

| Area | Current scope |
|---|---|
| Core built-ins | `eng.path`, `eng.io`, `eng.fs`, `eng.log`, `eng.process`, `eng.test`, and `eng.config` expose supported or supported-narrow compiler/runtime behavior. |
| Native workflow support | `eng.table`, `eng.timeseries`, `eng.stats`, `eng.sampling`, `eng.case`, `eng.artifact`, `eng.review`, `eng.model`, `eng.db`, `eng.net`, `eng.cache`, `eng.quality`, `eng.template`, `eng.workflow`, `eng.report`, `eng.plot`, and `eng.uncertainty` preserve typed workflow records, artifacts, and review metadata in the documented scopes. |
| Planned/internal boundaries | `eng.building`, `eng.system`, and `eng.ml` remain planned or internal unless a current status document says otherwise. |

## Source Files

Current `stdlib/eng/*.eng` files are module boundary notes. They are useful for
definition lookup and editor navigation, but a file existing under `stdlib/eng`
does not by itself mean that every helper is importable or stable.

Generated stdlib reference output should come from static metadata extraction,
not from executing EngLang workflows.
