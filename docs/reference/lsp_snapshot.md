# LSP Snapshot Reference

`eng-lsp.exe --snapshot <file.eng>` emits a compact JSON document for editor,
IDE, and automated smoke-test consumers that need compiler diagnostics,
completion metadata, and hover metadata without starting a long-lived LSP
session.

The current format marker is:

```json
{
  "format": "eng-lsp-snapshot-v1"
}
```

## Stability Policy

`eng-lsp-snapshot-v1` is experimental while v1.3 is not release-supported, but
it has a clear compatibility rule so tools can be built against it safely:

- Existing top-level keys keep their current type within `eng-lsp-snapshot-v1`.
- Existing required item keys keep their current type within
  `eng-lsp-snapshot-v1`.
- New optional keys may be added to top-level objects or item objects.
- Consumers must ignore unknown keys.
- A removal, required type change, semantic reinterpretation, or incompatible
  range convention requires a new format marker such as
  `eng-lsp-snapshot-v2`.
- `--snapshot-check <file.eng>` is the smoke path for this contract. It verifies
  that a supported example exposes non-empty completion and hover data without
  printing the full JSON.

## Top-Level Shape

```json
{
  "format": "eng-lsp-snapshot-v1",
  "diagnostics": [],
  "completions": [],
  "hovers": []
}
```

`diagnostics`, `completions`, and `hovers` are always arrays. The arrays may be
empty when the source legitimately has no diagnostics or no semantic metadata.

## Diagnostics

Diagnostics are shaped like LSP diagnostics:

```json
{
  "range": {
    "start": { "line": 0, "character": 0 },
    "end": { "line": 0, "character": 1 }
  },
  "severity": 1,
  "source": "eng",
  "code": "E-DIM-ADD-002",
  "message": "Cannot add or subtract DimensionlessNumber and Power.\nhelp..."
}
```

Current ranges are line-based and intentionally coarse: `line` is zero-based,
`character` is currently `0..1`. Precise character spans are deferred until the
compiler exposes consistent spans for all diagnostics.

Severity follows LSP numeric severity:

```text
1 = error
2 = warning
3 = information
```

## Completions

Completion items use LSP-style numeric kinds:

```json
{
  "label": "HeatRate",
  "kind": 7,
  "detail": "canonical unit W"
}
```

Current snapshot completions are global for the file and include:

- EngLang keywords
- current typed bindings
- schema columns
- domain names, domain variables, component names, and `Component.port` labels
- quantity kinds
- units

Live `textDocument/completion` requests may additionally use cursor context.
For example, `sensor.` returns the schema columns for the CSV promotion bound to
`sensor`.

## Hovers

Hover entries are derived from compiler hover metadata and selected semantic
metadata:

```json
{
  "name": "Q_coil",
  "kind": "variable",
  "line": 28,
  "quantity_kind": "TimeSeries[Time] of HeatRate",
  "display_unit": "W",
  "status": null,
  "contents": {
    "kind": "markdown",
    "value": "**Q_coil**\n\ninferred as TimeSeries[Time] of HeatRate [W]..."
  }
}
```

`line` is one-based because it mirrors compiler metadata. LSP responses convert
request positions from zero-based LSP coordinates before matching hover lines.
`quantity_kind` and `display_unit` are included for editor clients that want to
render compact metadata without parsing the markdown body.

`kind` and `status` are optional metadata extensions inside
`eng-lsp-snapshot-v1`; consumers should continue to ignore unknown keys.
Current hover kinds include:

```text
variable
domain
domain_variable
domain_conservation
component
component_port
connection
```

For v2.0 domain/component files, hovers expose domain declarations,
across/through variables, conservation metadata, component ports, and connection
status. Example connection hover:

```json
{
  "name": "RoomBoundary.heat -> AmbientBoundary.heat",
  "kind": "connection",
  "line": 26,
  "quantity_kind": "Thermal",
  "display_unit": "-",
  "status": "domain_compatible"
}
```

## Intended Consumers

Use the snapshot for:

- package and CI smoke checks
- native IDE metadata inspection
- extension tests that do not need a persistent server
- debugging compiler metadata quickly from the command line

Use the stdio LSP server for:

- real editor clients
- unsaved buffer diagnostics
- cursor-position completion
- hover requests in an editor

The snapshot is not a replacement for full LSP editor validation.
