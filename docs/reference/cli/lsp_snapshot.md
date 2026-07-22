# Advanced Editor Metadata JSON

This page is for maintainers and editor-tooling authors. Normal users should
prefer the VS Code extension, native IDE, or `eng.exe check`.

This page documents two maintainer JSON surfaces:

- `eng-lsp.exe --snapshot <file.eng>` emits live document data for editor, IDE,
  and automated smoke-test consumers that need compiler diagnostics,
  evaluated validation metadata, completion metadata, hover metadata, semantic
  highlighting data, document symbols, and folding ranges without starting a
  long-lived editor session.
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
  "validations": [],
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

`diagnostics`, `validations`, `completions`, `hovers`, `document_symbols`, and
`folding_ranges` are always arrays. `semantic_tokens` always has a legend and a
token array. These arrays may be empty when the source legitimately has no
diagnostics or no semantic metadata.

## Validation Results

`validations` uses the same compiler-owned records as
`review.json.review_document.validations`. Every row identifies its `kind`,
`target`, `expression`, `evaluation_phase`, `status`, and one-based
`source_span`. Command validations use `pending_runtime` after successful
lowering because their outcome is known only when the workflow runs. Class
rules use `declared`; class-object rows use `pass`, `fail`, or `unresolved` and
include observed values when available. Object rows point `source_span` at the
object declaration and preserve the class rule location in `rule_source_span`.
This distinction lets editors show evaluated results without presenting a rule
declaration as though it had passed for every object. Each span also emits
`source_origin` as `root` or `import`. A root object using an imported class can
therefore have a root `source_span` and an import `rule_source_span`; clients
must not navigate an import-owned line/column pair inside the checked root
buffer. Import-owned spans include `source_path`, relative to the root source
directory when possible, so clients can open the owning file. Imported
class-rule rows remain available as program-level validation metadata with the
same explicit origin.

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

`line` is zero-based. `character` uses LSP UTF-16 offsets. A compiler-owned
`Diagnostic::source_span` is authoritative and is converted from source-byte
coordinates to UTF-16. This covers precise `with` option key/value ranges,
typed state-space arguments, workflow expressions, declarations, operators,
function calls, and source operands. Function return diagnostics use the exact
block or inline return expression; missing-return diagnostics use the function
name because there is no expression. Component assembly balance and
algebraic-loop diagnostics use the first source component name as their stable
anchor. Unconnected-port diagnostics use the port name, and invalid port-domain
diagnostics use the complete domain reference. A corpus gate scans all 153
example, diagnostic, and grammar-fixture `.eng` files and requires every
diagnostic in these assembly/port classes to retain a valid compiler range.
Domain declarations additionally preserve generic kind/name, variable type/unit,
and conservation ranges. Component declarations preserve parameter annotations
and defaults, local expressions and equation sides, constructor values, and both
connect endpoints; system equations preserve both source sides. Domain contract,
connect compatibility, component parameter/equation/boundary/behavior, and
physical-equation diagnostics use the failing declaration, endpoint, call,
argument, unit, or equation side. Their dedicated corpus guard rejects any
return to inferred ranges.
Invalid network URL diagnostics use the complete request or download URL operand.
Declared URL aliases are resolved before validation, while an `args.*` URL that
has not been supplied yet is not presented as malformed. URL ranges end before
trailing `#` or `//` comments. All `E-ML-SOURCE-*` and `E-ML-ARGS-*`
diagnostics use compiler-owned ML expression, source operand, option key/value,
or individual feature spans. Inline named arguments and attached `with` blocks
share this range model, and trailing option comments are excluded.
`W-ML-TRAIN-ALIAS` selects only the `regression_table` or `train_regression`
call name so deprecation styling and migration actions do not underline the
model operands. The corpus
also requires every `E-UNC-SOURCE-*` and `E-UNC-ARGS-*` diagnostic to retain its
uncertainty expression, source, positional value, or named value range. Nested
constructor values and trailing comments keep their source boundaries. Every
`E-SAMPLING-*` diagnostic also retains either its option value or owning
`sample <method>` expression range. `E-UNC-DIRECT-COMPARE` selects the uncertain
operand, `E-UNC-PERCENTILE-UNIT-MISMATCH` selects the incompatible threshold,
`E-UNC-PROBABILITY-EXPR-INVALID` selects the complete probability call, and
`E-VALIDATE-UNIT-001` selects the right comparison operand. A global
simulation/solver corpus guard also requires every observed `E-SIM-*` and
`E-SOLVE-*` diagnostic to retain a compiler-owned range: unknown targets select
the target name, missing required options select the owning `simulate`/`solve`
RHS, and malformed supplied options select their exact value. Their value quick
fixes prefer that Problems range, while a missing single option is inserted into
the attached `with` block or a newly created block. Every `E-CLASS-*` diagnostic
also has a compiler-owned range. Field defaults and object assignments select
their value, invalid validation and method return declarations select their
expression, missing fields select the object name, and method-call diagnostics
select the receiver, method, or argument. A dedicated promotion guard requires
schema lookups to select the schema name and CSV/JSON/config source and
validation failures to select the source operand. Unreadable sources do not
cascade into missing-column or missing-field diagnostics. Process commands, DB
connections/tables, CSV export sources/fields, write formats/targets, print/log
interpolation, and nested file operations likewise use parser-owned keyword or
operand ranges. Their corpus guard requires all 18 observed diagnostics in
these classes to retain a compiler range. Quantity ambiguity selects the direct
literal unit, HeatRate sum warnings select `sum`, dimensionless arithmetic
selects the offending `+` or `-`, and function-call failures select the called
name or failing argument. Their fixture guard requires every observed
`W-QTY-AMBIG-*`, `W-STATS-SUM-*`, `E-DIM-ADD-*`, and `E-FN-CALL-*` diagnostic
to retain a compiler range. Derivative duplicates select `der(<state>)`; legacy
row selection selects `select_first_row`; missing table-join policy selects the
join expression; run-case source and option errors select the source or option
key; deprecated root/test declarations select their source keyword/header; and
implicit TimeSeries fill warnings select the fill expression. A missing
run-case `results` map produces one diagnostic on its owner expression rather
than duplicate messages. The global 153-file corpus gate requires zero uses of
the older range-inference path. That inference code remains only as a defensive
compatibility path for diagnostics outside the checked compiler corpus.
Diagnostics owned by an imported file are not published against the checked
root document. They are reported when that imported document is checked as its
own LSP text document, preventing valid imported byte offsets from underlining
unrelated root text.

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
- state-space vector types and their bare plus `Type.member` labels
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
  "source_origin": "root",
  "source_path": null,
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
`source_origin` is `root` or `import`. Imported symbol metadata remains in the
snapshot so name-resolved hover and cross-file navigation can describe imported
definitions. Root-owned structured metadata wins when a root declaration and an
imported definition share a name; anonymous same-line fallback accepts only
root-owned hover entries. Imported entries include a root-relative
`source_path` when possible and show that path in their Markdown body; root
entries use `null` because the request URI already identifies their file.
`quantity_kind` and `display_unit` are included for editor clients that want to
render compact metadata without parsing the markdown body. A semantic-role
hover that does not describe a physical quantity leaves `quantity_kind` empty;
its Markdown body omits the Quantity row instead of displaying an empty value.

`kind` and `status` are optional metadata extensions inside
`eng-lsp-snapshot-v1`; consumers should continue to ignore unknown keys.
The Markdown body renders user-facing kind/status labels while these JSON
fields keep their stable raw ids for clients that match on them.
Current hover kinds include:

```text
variable
state_space_type
state_space_member
function
function_local
where_local
domain
domain_variable
domain_conservation
component
component_port
connection
component_assembly
connection_set
assembly_equation
class
class_field
class_validation
class_method
class_object
object_field
object_validation
unit
quantity
schema_field
timeseries_axis
timeseries
side_effect
external_boundary
uncertainty
validation
http_response_field
coverage_result_field
time_alignment_result_field
table_field
sample_table_field
db_connection_field
case_table_field
case_output_table_field
case_run_result_table_field
case_result_collection_table_field
model_field
prediction_table_field
```

Component hover/completion details distinguish a `component template` from a
`component instance of Name`. Both remain present when a source declares a template
and constructs one or more system-local instances.

Declaration tokens for import targets, const names/types/units, function names,
parameter names/types/units, function return types/units, schema columns,
state-space type blocks and members, system variables and state vectors, domain
variables, component ports, parameters, inputs and locals, class fields and
methods, args fields, and class object bindings and fields use parser-owned
source ranges. Every compiler-span helper rejects import-owned ranges before
using offsets or attempting a guarded line fallback. Imported definitions still
participate in symbol resolution, so references written in the root document
retain the correct function/type/readonly role without receiving imported
declaration or definition modifiers.

Domain generic kinds and named parameters are emitted from their own spans.
Domain variables and component parameters/inputs also emit quantity and explicit
unit tokens from their annotation ranges. Each resolved connection endpoint is
split inside its compiler range into a model/solver variable component token and
model/solver property port token, so repeated endpoint names elsewhere on the
line cannot be recolored.

System/component type and unit tokens, typed state-space vector type
expressions, and linear-operator type expressions also use parser-owned ranges.
State-space vector and operator type identifiers carry the `solver` modifier,
and `operator Name:` marks `Name` as the declaration rather than the keyword.

Structural references on those declarations also use parser-owned ranges: schema,
class, and args types; schema/class units; component-port domains; object-literal class
names; and copy-with source objects. Generic type expressions are emitted as
separate identifier tokens, so punctuation, whitespace, and nested type arguments
do not create overlapping semantic-token ranges.

State-space blocks are user-defined type declarations. Their names receive
`class` tokens with `state`, `input`, or `output` plus `solver`; members receive
role-matched `property` declarations; and member type/unit tokens are restricted
to their parser-owned ranges. `StateVector[Name]`, `InputVector[Name]`, and
`OutputVector[Name]` references participate in definition, references, and safe
rename without treating a same-spelled member or quantity type as the block name.

Compiler symbol metadata remains the preferred hover source. When a semantic
token has no matching symbol hover, the snapshot adds a role hover for units,
quantities, declared fields, TimeSeries axes/operations, side effects, external
boundaries, uncertainty, and validation. These fallback entries preserve the
exact semantic token type/modifiers in their detail text, so the VS Code and
native IDE clients can explain role coloring without duplicating compiler
classification logic.
Editor clients resolve these entries from semantic-token UTF-16 ranges, not
identifier-only word boundaries, so composite units such as `W/m2` and
`W/(m2*K)` receive the same hover as simple names such as `degC`.

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
that need efficient live highlighting should use the stdio semantic-token
requests. The server advertises full tokens with delta support plus range
tokens. A `textDocument/semanticTokens/full` response includes an opaque
`resultId`; pass it as `previousResultId` to
`textDocument/semanticTokens/full/delta`. The server returns a minimal integer
stream edit when that result is still cached and a complete `data` response
when the result is unknown or expired. Delta history is bounded per document
and follows the latest unsaved buffer.

Scoped `where`/`with` opener tokens, `where` local declarations, and inline
`with` option keys use compiler-owned spans. Option list/enum values and path
helpers are searched only inside the corresponding option value span, so a
matching word in an earlier option or string does not receive the later role.

Unquoted import targets use their full compiler-owned span as one atomic
`namespace` token, including dotted targets such as `eng.stats`. Review-risk
metadata merges into the symbol that starts at the first non-keyword identifier;
if that symbol does not exist, the compatibility variable fallback is retained
instead of attaching the risk to a later type or unit token.

Simple identifier sources in `write` statements use the compiler-owned write
expression span. If the identifier is also spelled like a language keyword, the
exact source range emits one variable token with its binding and write-context
modifiers; unrelated grammar occurrences keep their keyword token.

Simple inferred aliases and ML source/input operands also use compiler-owned
expression ranges. This keeps soft-keyword bindings such as `model` and
`records` variable-colored in resolved value positions, merges their model or
workflow modifiers into one token, and leaves actual grammar and member
positions independently classified. Dotted ML operands and features emit one
token per identifier, with `args` as a parameter, other receivers as variables,
and member segments as properties. Command-style `apply` targets follow the same
rule and classify the final segment as a workflow-step function.

`promote csv/json/toml` and `promote json records` bindings, source operands,
and schema targets use compiler-owned token ranges. A repeated identifier on the
same line can therefore be a declaration variable, an external source path, and
a schema class without overlapping semantic tokens. `file`/`dir`/`join` helpers
are searched only inside the source span.

Print/log templates, CSV export sources and fields, writes, DB reads, nested file
operations, and process runs also use compiler-owned side-effect metadata.
Keywords, source expressions, format/unit fragments, paths, connection/table
segments, bindings, and commands are projected only from their exact ranges.
The DB metadata consumed here is the same structured target consumed by native
runtime execution, not an editor-only reconstruction.

Sampling declarations and distribution option names use compiler-owned binding
and key spans. Their semantic token and Outline selection ranges therefore point
to the exact declaration occurrence instead of searching the source line.

Class method return types and explicit units use compiler-owned signature spans
for quantity/unit tokens. Class, validation, method, object, and explicit
object-field Outline selections use their declaration or expression spans;
copy-with objects do not repeat inherited fields that would select the source
object's lines. Evaluated object validation results remain in `validations`
rather than becoming document-symbol children at the class rule's line.

Command-style targets and clause names/values also use compiler-owned spans.
Their identifiers and clause keywords stay inside the owning command expression,
including repeated text and trailing comments.

Uncertainty declarations, source operands, and named constructor arguments also
use compiler-owned spans. Named keys are uncertain properties; distribution
kind and propagation method values are uncertain keywords; and dotted named
values retain parameter/receiver/property segmentation. Tokens are constrained
to the owning argument range, so a binding named `method` or `kind` remains one
declaration token even when the same word appears as an option key.

Lexical number tokens require identifier boundaries and include valid decimal
exponents. Operator tokens are excluded from those numbers, generated
hyphenated workflow literals, catalog units, and exact compiler-owned unit
spans. Outer string tokens are split around interpolation parameters/properties,
format precision numbers, display units, and quoted import namespaces, so the
nested roles remain independently colorful without overlapping their string
container. The snapshot coverage gate rejects every overlapping semantic-token
pair across all examples and grammar fixtures, including equal-type and
whole-path/segment overlaps.

## Document Symbols And Folding Ranges

`document_symbols` uses LSP-style document symbol JSON with numeric symbol
kinds, source ranges, selection ranges, details, and children. It is intended
for outlines and breadcrumbs.

Every emitted symbol stores explicit UTF-16 selection start and end coordinates
from its compiler `SourceSpan`. The end is not inferred from the displayed
symbol name, which may be synthetic or longer than the selected source token.
The active path no longer searches a source line for a same-spelled name.
Schema/state-space/table/network/assembly/args/test/golden/expectation,
`where`/`with`, and typed-binding symbols all use compiler-owned spans.

Only spans owned by the checked root source are emitted. Definitions imported
from another file remain available to definition/reference features but do not
appear at unrelated ranges in the current document Outline. A recursive corpus
gate checks every Outline symbol across examples, diagnostic fixtures, and
grammar fixtures for a nonempty UTF-16 selection contained by its symbol range.

Import target and const name selection ranges come from compiler-owned spans,
including CRLF sources and targets containing non-BMP characters. Const type
and explicit unit ranges use the same spans as semantic highlighting, and
import/const Problems ranges underline the target or expression rather than the
declaration keyword.

Function and parameter selection ranges also come from compiler-owned name
spans, so a parameter that repeats its function name selects the parameter
occurrence. Parameter and return type/unit colors share their parser spans with
unknown-type Problems ranges and remain correct after non-BMP text in a generic
type expression.

`where` local and `with` option children select their exact compiler-owned name
or key spans instead of the first matching word on the source line. This remains
correct for CRLF input and for inline blocks containing non-BMP text before the
selected option.

Sampling bindings and distribution children use their exact compiler-owned name
and option-key spans for Outline selection.

CSV, JSON-record, and config promotion symbols select their exact binding spans;
their details retain the promoted schema name.

Table-transform and bound HTTP-request symbols likewise select their exact
binding spans. Args containers select `args`, while fields select their declared
names. Test containers and golden children select string content without quote
delimiters; expectation suites select `expect` and expectation children select
their first subject token.

CSV export and process symbols select their exact source or command-owned
ranges. Export field children select the expression occurrence that owns the
field, including repeated dotted paths and non-BMP text before the selection.

Domain symbols select the declaration name; generic parameters, variables, and
conservation children select their own source occurrences. Component ports,
parameters, inputs, locals, and source equations likewise use compiler spans,
and each connection has a source symbol selecting its left endpoint. System
equations select their left expression. Synthesized residual records remain in
semantic metadata but are omitted from the structural Outline.

Class and object symbols select their exact names. Their field and method
children do the same, while a class validation child selects its rule
expression.

Command symbols select their target span. Assertion children select the exact
comparison operator, with operand fallback for incomplete assertions, rather
than the first same-spelled text on the line.

Each state-space vector type is a top-level struct-like symbol whose member
fields are nested children with role, quantity type, and unit detail.

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
- go-to-definition for current-file, unsaved-aware static-import, and bundled
  stdlib symbols
- document symbols, workspace symbols, folding ranges, and semantic tokens
- same-symbol document highlights and static-import-aware Find All References
- safe current-file and static-import-aware workspace rename

Start the persistent server with `eng-lsp --stdio` (a no-argument invocation is
also accepted). The VS Code extension initializes one process with every open
workspace folder, sends full-text `didOpen`/`didSave`, incremental UTF-16 range
`didChange`, and `didClose` notifications, consumes versioned
`publishDiagnostics`, and sends
`$/cancelRequest` when VS Code cancels a provider request. Semantic highlighting
uses `textDocument/semanticTokens/full` directly. The extension-only
`englang/snapshot` request returns `eng-lsp-snapshot-v1` review and decoration
metadata for an already-open document over the same stdio connection; it is
advertised as `capabilities.experimental.englangSnapshotProvider`. A
`workspace/didChangeWatchedFiles` notification invalidates semantic-token
history and republishes diagnostics so closed imported files changed on disk do
not leave open dependents stale.

`textDocument/references` always analyzes the current unsaved document and
honors `context.includeDeclaration`. For an importable `const`, function,
schema, class, system, state-space vector type, domain, or component, it also
searches open documents and
saved `.eng` files under the initialized workspace roots. A candidate file is
included only when its static file-import chain resolves the name to the same
declaration file; unrelated same-name symbols are excluded. Open document text
takes precedence over its saved file. The scan is bounded to 500 files and
1,000 locations. Local variables, parameters, and members remain document
scoped.

The single-buffer compatibility form is
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

The single-buffer compatibility rename form is
`--rename-stdin <file.eng> <line> <character> <new-name> [workspace-root]`.
Without `workspace-root`, declarations that are safe to rename in the current
buffer remain current-file operations; selecting an imported symbol returns an
actionable error instead of an incomplete edit.

The single-buffer compatibility definition form is
`--definition-stdin <file.eng> <line> <character>`. It reads the selected
buffer from stdin and unchanged imported files from disk. Use the workspace
form when other open imports may be modified.

The equivalent single-buffer live-analysis compatibility forms are
`--snapshot-stdin <file.eng>` and
`--completion-stdin <file.eng> <line> <character>`. Both read only the selected
buffer from stdin and resolve imports from disk.

Compatibility clients with multiple modified buffers use the workspace forms:

```text
--workspace-snapshot-stdin <workspace-root> <file.eng>
--workspace-completion-stdin <workspace-root> <file.eng> <line> <character>
--workspace-definition-stdin <workspace-root> <file.eng> <line> <character>
--workspace-references-stdin <workspace-root> <file.eng> <line> <character> [true|false]
--workspace-prepare-rename-stdin <workspace-root> <file.eng> <line> <character>
--workspace-rename-stdin <workspace-root> <file.eng> <line> <character> <new-name>
```

All six read the same strict JSON payload from stdin. Workspace snapshot emits
the full `eng-lsp-snapshot-v1` object; workspace completion emits its bounded
position-aware `completions` payload:

```json
{
  "format": "eng-lsp-open-documents-v1",
  "documents": [
    { "path": "C:/workspace/main.eng", "source": "use \"module.eng\"\n..." },
    { "path": "C:/workspace/module.eng", "source": "const RATE = 0.8\n" }
  ]
}
```

The selected file must be present. Other entries should be modified open
`.eng` documents; omitted workspace files are read from disk. Paths must resolve
to existing files inside `workspace-root`, duplicates are rejected, and open
text takes precedence throughout the static-import graph. The contract accepts
at most 128 documents, 4 MiB per document, and 16 MiB of document source in
total, inside a 100 MiB serialized payload limit. Editor clients must discard
the result if the participating document set, dirty state, or version changes
while the subprocess is running. Persistent LSP clients provide open document
versions through `didOpen` and ordered incremental `didChange` events; editor
snapshots and position requests resolve imports from that full open-document
set, and a document change republishes diagnostics for open dependents.
`didClose` removes that buffer override, clears its diagnostics, and rechecks
open dependents against the remaining open buffers and saved files.

The incremental protocol mirror reduces editor-to-server transfer size and
applies multiple changes in order without splitting UTF-16 surrogate pairs or
CRLF boundaries. Most semantic edits still build a complete `CheckReport` for
the affected root and open dependents. A bounded compiler-owned path can instead
preserve an unchanged, verified report prefix and reparse and semantically
reanalyze only a final suffix of supported scalar declarations. The preserved
prefix may include an unchanged scalar-only import environment or, under stricter
clean-report and vector-ownership checks, richer top-level constructs such as a
file/path or TimeSeries binding, cached boundary, pure scalar helper, and
`print`. Old and new suffix results must remain registered scalars; each
patched semantic vector must expose an exact independent tail; and prior cache
and axis metadata must regenerate exactly. A successful patch rebuilds axis
metadata and rekeys cache records with the new source hash. Suffix expressions
cannot use a preserved non-scalar binding as an alias or operand. Changes inside
that richer prefix and token-bearing non-declarations in the affected suffix use
full analysis. This richer root-edit contract may retain unchanged supported
`eng.*` module and static file imports after exact root line, span, kind, and
status verification. Static imports additionally require the complete recursive
path-to-source-ID registry to reproduce exactly. Preserved imported semantic
definitions are limited to schemas, constants, and functions whose internal
spans retain registered source ownership; imported systems, domains, components,
and classes use full analysis. Only root import declaration lines are reparsed
for verification; supported imported definitions and other richer-prefix
constructs are preserved without reparsing or semantic reanalysis.
Open or watched import changes invalidate cached dependent reports before this
path is considered. This is not general incremental parsing or semantic
recomputation.

This JSON contract is not a replacement for full LSP editor validation.
