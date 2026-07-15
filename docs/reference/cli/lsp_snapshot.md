# Advanced Editor Metadata JSON

This page is for maintainers and editor-tooling authors. Normal users should
prefer the VS Code extension, native IDE, or `eng.exe check`.

This page documents two maintainer JSON surfaces:

- `eng-lsp.exe --snapshot <file.eng>` emits live document data for editor, IDE,
  and automated smoke-test consumers that need compiler diagnostics,
  completion metadata, hover metadata, semantic highlighting data, document
  symbols, and folding ranges without starting a long-lived editor session.
- `eng-lsp.exe --editor-metadata` emits the static editor catalog used by the VS
  Code extension, grammar smoke checks, and native IDE bootstrap contracts.

The `--snapshot` format marker is:

```json
{
  "format": "eng-lsp-snapshot-v1"
}
```

## Compatibility Policy

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
- `--snapshot-check <file.eng>` is the smoke command for this contract. It
  verifies that a supported example exposes non-empty completion and hover data
  without printing the full JSON.

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

## Static Editor Metadata

`--editor-metadata` uses this format marker:

```json
{
  "format": "eng-lsp-editor-metadata-v2"
}
```

It contains:

- `semantic_token_legend`: token types and modifiers shared with VS Code.
- `syntax_catalog`: keyword, workflow builtin, with-option, public type,
  quantity, compiler unit labels, public workflow member field catalogs such as
  HTTP/sample/DB/case/model/prediction fields, highlight-only legacy workflow
  aliases, and highlight-only legacy unit aliases used by grammar, local
  completion fallback, and highlight checks.
- `completion_items`: fallback completions used when live completion is
  unavailable. Rust callers should use `editor_completion_items()`.

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

Current metadata completions are global for the file and include:

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
The Markdown body renders user-facing kind/status labels while these JSON
fields keep their stable raw ids for clients that match on them.
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

The metadata token list is suitable for preview/debug tooling. Editor clients
that need efficient live highlighting should use the stdio
`textDocument/semanticTokens/full` request.

## Document Symbols And Folding Ranges

`document_symbols` uses LSP-style document symbol JSON with numeric symbol
kinds, source ranges, selection ranges, details, and children. It is intended
for outlines and breadcrumbs.

`folding_ranges` contains zero-based line ranges plus an optional `kind`
(`comment`, `imports`, or `region`) for editor folding support.

## Intended Consumers

Use this JSON contract for:

- package and CI smoke checks
- native IDE metadata inspection
- extension tests that do not need a persistent server
- debugging compiler metadata quickly from the command line
- semantic token and TextMate fallback verification

Use the stdio LSP server for:

- real editor clients
- unsaved-buffer diagnostics, hover, completion, quick fixes, and formatting
- go-to-definition for current-file, static-import, and bundled stdlib symbols
- document symbols, workspace symbols, folding ranges, and semantic tokens
- same-symbol document highlights and static-import-aware Find All References
- safe current-file and static-import-aware workspace rename

`textDocument/references` always analyzes the current unsaved document and
honors `context.includeDeclaration`. For an importable `const`, function,
schema, class, system, domain, or component, it also searches open documents and
saved `.eng` files under the initialized workspace roots. A candidate file is
included only when its static file-import chain resolves the name to the same
declaration file; unrelated same-name symbols are excluded. Open document text
takes precedence over its saved file. The scan is bounded to 500 files and
1,000 locations. Local variables, parameters, and members remain document
scoped.

The on-demand CLI form is
`--references-stdin <file.eng> <line> <character> [true|false] [workspace-root]`.
Without `workspace-root`, it returns current-buffer occurrences plus occurrences
in the resolved declaration file; the wider saved-file scan is disabled.

`textDocument/rename` keeps local variables and parameters document scoped. For
an importable top-level `const`, function, schema, class, system, domain, or
component, it can instead return one workspace edit covering the declaration
and every open or saved `.eng` file whose static file-import chain resolves to
that exact declaration. Unrelated same-name symbols are excluded, and open
document text takes precedence over disk. The selected file and declaration
must both be inside an initialized workspace root. The entire rename is rejected
if the 500-file scan is truncated, a required source is unreadable, semantic
occurrence coverage is incomplete, a name conflict is found in any affected
file, or the edit would exceed 1,000 locations. Built-ins and members remain
non-renameable.

The on-demand rename form is
`--rename-stdin <file.eng> <line> <character> <new-name> [workspace-root]`.
Without `workspace-root`, declarations that are safe to rename in the current
buffer remain current-file operations; selecting an imported symbol returns an
actionable error instead of an incomplete edit. The CLI receives only the
current unsaved buffer, so editor clients must save or otherwise account for
other modified documents before requesting a workspace rename. Persistent LSP
clients can supply all open document versions directly.

This JSON contract is not a replacement for full LSP editor validation.
