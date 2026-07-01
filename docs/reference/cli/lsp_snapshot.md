# LSP Snapshot Reference

`eng-lsp.exe --snapshot <file.eng>` emits a compact JSON document for editor,
IDE, and automated smoke-test consumers that need compiler diagnostics,
completion metadata, hover metadata, semantic tokens, document symbols, and
folding ranges without starting a long-lived LSP session.

The current format marker is:

```json
{
  "format": "eng-lsp-snapshot-v1"
}
```

## Stability Policy

`eng-lsp-snapshot-v1` is experimental and not public release-supported, but
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
  "hovers": [],
  "semantic_tokens": {
    "legend": {
      "token_types": [],
      "token_modifiers": []
    },
    "tokens": []
  },
  "document_symbols": [],
  "folding_ranges": []
}
```

`diagnostics`, `completions`, `hovers`, `document_symbols`, and
`folding_ranges` are always arrays. `semantic_tokens` always has a legend and a
token array. These arrays may be empty when the source legitimately has no
diagnostics or no semantic metadata.

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

`line` is zero-based. `character` uses LSP UTF-16 offsets. Diagnostics use
source-aware ranges when possible: dimensionless arithmetic diagnostics
highlight the offending `+` or `-`, schema fast-assignment diagnostics
highlight `=`, and file mutation diagnostics target `move` or `delete`. Generic
diagnostics fall back through backticked message/help text, then the first
identifier or visible token on the line.

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
- function names with signature details
- schema columns
- domain names, domain variables, component names, and `Component.port` labels.
  Generic ports include canonical labels such as `Fluid[Water]` in `detail`.
- class names, `Class.field` labels, methods, and object member labels. Class
  field completion details mark required fields and default values.
- quantity kinds
- units

Live `textDocument/completion` requests may additionally use cursor context.
For example, `sensor.` returns the schema columns for the CSV promotion bound to
`sensor`. `building.` returns fields and zero-argument metadata methods from
the object's class. Inside an object literal or copy-with block, completion
returns the remaining class fields with `required` or `default = ...` detail.

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
function
function_local
where_local
domain
domain_variable
domain_conservation
component
component_port
connection
class
class_field
class_validation
class_method
class_object
object_field
object_validation
```

For domain/component track files, hovers expose domain declarations,
across/through variables, conservation metadata, component ports, and connection
status. Domain hovers may include package/version metadata and generic
parameter signatures such as `Fluid[Medium M]`; port hovers include the port
type, base domain, and labeled metadata such as `medium Water`, `frame World`,
or `axis X` when generic domain arguments are present. Example connection hover:

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

Class/object hovers expose class declarations, field units/defaults/required
status, object literal fields, copy-with fields, validation results, and
zero-argument metadata methods. Field hover details use the same required/default
wording as completion details so editor clients can render compact field lists
without reading `class_summary` directly.

Function hovers expose the full signature, return quantity/unit, return
expression when available, and function-local binding names. `function_local`
hover entries are scoped as `function.local` so editor clients can distinguish
function-local symbols from importable or top-level bindings.

`where_local` hovers expose owner-local `where { ... }` bindings as
`where.<name>` with inferred quantity/unit, owner line, expression, and status.

## Semantic Tokens

Semantic token output mirrors the stdio LSP legend used by the VS Code
extension:

```json
{
  "semantic_tokens": {
    "legend": {
      "token_types": ["namespace", "type", "class", "interface"],
      "token_modifiers": ["declaration", "defaultLibrary", "unit"]
    },
    "tokens": [
      {
        "line": 0,
        "start": 0,
        "length": 6,
        "type": "keyword",
        "modifiers": ["declaration"]
      }
    ]
  }
}
```

Token coordinates are zero-based and use UTF-16 offsets. The current legend is
validated by `ide-check` against the VS Code extension's semantic token arrays
and package manifest so LSP-only or extension-only modifier drift fails the
editor contract gate.

Current semantic token roles include:

- standard syntax roles such as namespace, type, class, parameter, variable,
  property, function, method, keyword, operator, string, number, and comment
- EngLang modifiers for units, quantities, axes, TimeSeries values,
  uncertainty, reports, validations, side effects, external boundaries, inputs,
  state, model, DB, cache, workflow steps, internal/planned symbols, and review
  risk

The snapshot token list is suitable for preview/debug tooling. Editor clients
that need efficient live highlighting should use the stdio
`textDocument/semanticTokens/full` request.

## Document Symbols And Folding Ranges

`document_symbols` uses LSP-style document symbol JSON with numeric symbol
kinds, source ranges, selection ranges, details, and children. It is intended
for outlines and breadcrumbs.

`folding_ranges` contains zero-based line ranges plus an optional `kind`
(`comment`, `imports`, or `region`) for editor folding support.

## Intended Consumers

Use the snapshot for:

- package and CI smoke checks
- native IDE metadata inspection
- extension tests that do not need a persistent server
- debugging compiler metadata quickly from the command line
- semantic token and TextMate fallback verification

Use the stdio LSP server for:

- real editor clients
- unsaved buffer diagnostics
- cursor-position completion
- hover requests in an editor
- conservative line-based go-to-definition for symbols whose definition line is
  in the current document

The snapshot is not a replacement for full LSP editor validation.
