# Compiler Frontend

The compiler frontend turns `.eng` source into compiler-owned program data used
by the CLI, runtime, report generator, LSP, VS Code extension, and native IDE.
It is the shared path for diagnostics, review data, and editor metadata.

```text
source text
  -> SourceLine / SourceSpan
  -> lexer tokens
  -> parser items
  -> semantic analysis
  -> CheckReport
  -> runtime, review, report, and editor payloads
```

## Source Spans

`SourceSpan` is the source location contract for tokens, parsed items,
diagnostics, hovers, semantic tokens, document symbols, and quick fixes.

```text
start   source byte offset
end     source byte offset
line    1-based line number
column  1-based column number
source_id  compiler-owned source identity; 0 is the checked root buffer
```

The CLI, LSP, VS Code extension, and native IDE all rely on these spans so that
Problems ranges, underlines, hover locations, and highlight inspection rows point
at the same source text.

Every public `AstItem` exposes `primary_span()`, an exhaustive parser-owned
anchor across all item variants. The primary span is intentionally not named a
full node extent: depending on the grammar item it identifies the declaration
keyword, declared name, operand, or complete scoped option. Variant records
retain the narrower name, type, unit, expression, key, and value spans used by
semantic analysis and editor tooling. A parser corpus gate checks that every
primary span is a non-empty UTF-8 byte range with exact line, byte-column, and
root/import source ownership.

`Diagnostic::source_span` remains optional in the serialized type for
diagnostics that have no source occurrence, but it is authoritative whenever a
check reports source code: review JSON uses its starting column and the LSP
converts its byte column and byte length to UTF-16 editor coordinates. A corpus
gate requires every diagnostic emitted by the 153 example, diagnostic, and
grammar-fixture files to provide this compiler-owned range; the current LSP
text-inference count is zero. `with` options preserve separate whole-option,
key, and value spans so option diagnostics do not need to search the source line
by wording. Line starts are counted from the original bytes, including two-byte
CRLF line endings.

Static file imports are lexed and parsed with a stable nonzero `source_id`
derived from their resolved path. Every child range preserves its parent's
identity through parsing and semantic lowering. Current-document diagnostics,
semantic declaration overlays, structural token classification, and Outline
generation reject import-owned ranges instead of applying another file's byte
offsets to the active buffer. Imported definitions remain available to
completion, reference, definition, and name-resolved hover features, which
route those files separately. Anonymous line-hover fallback considers only
root-owned metadata. Review validation spans likewise distinguish a root object
location from an imported class-rule location.

## Lexer And Parser

`lexer.rs` classifies comments, identifiers, keywords, numbers, string
literals, units, and symbols. `parser.rs` groups those tokens into declarations,
blocks, command-style workflow statements, expressions, object literals, and
legacy syntax markers that semantic analysis can diagnose precisely.

The parser records enough context to distinguish top-level statements, `args`,
`schema`, `where`, `with`, validation, report, class/object, workflow, and
solver-oriented blocks before semantic analysis attaches type, unit, artifact,
and editor metadata. Inline `with { ... }` parsing splits only at top-level
commas or semicolons, preserving separators inside calls, lists, nested objects,
and quoted strings.

Semantic metadata retains the parser-owned `where`/`with` keyword anchors in
`WhereBlockInfo.span` and `WithBlockInfo.span`, each `where` local name in
`WhereBindingInfo.span`, and each option key/value in `WithOptionInfo.key_span`
and `value_span`. Semantic highlighting and outline selection consume those
ranges directly, including inline blocks with repeated words in earlier options.

Side-effect declarations retain token-owned structure instead of requiring a
consumer to search or reparse their normalized expression text. `PrintDecl`,
`CsvExportDecl`, `WriteDecl`, `FileOperationDecl`, and `ProcessRunDecl` preserve
their operation keywords, operands, formats, paths, separators, and optional
interpolation fields. `DbTableTargetDecl` separates a DB connection path from
the quoted table name, and `FastBinding.db_read` records the corresponding
`read sqlite <connection>.table("...")` source structure before normalization.

The public `PrintInfo`, `CsvExportInfo`, `WriteInfo`, `FileOperationInfo`,
`ProcessRunInfo`, and `DbReadInfo` records carry those exact ranges through
semantic analysis. Diagnostics, semantic highlighting, Outline selection, and
native runtime DB reads/writes therefore consume one compiler-owned target and
source identity. A simple write-source identifier can still be classified from
its compiler role even when its spelling is also a workflow keyword, without
changing a real keyword occurrence elsewhere.

`FastBinding.expression_span` identifies the complete source RHS before later
semantic normalization. Successful inferred declarations retain that range.
Function-call parsing also retains byte ranges for the function name and every
top-level argument relative to that RHS. `E-FN-CALL-001` and
`E-FN-CALL-002` therefore select the called name, while argument type failures
select the exact failing argument even when another copy of the same text
appears in a string or trailing comment.

Expression-type diagnostics follow the same source-owned policy.
`W-QTY-AMBIG-001` selects the direct quantity literal's unit and does not infer
a binding's result quantity from string contents or function arguments.
`W-STATS-SUM-001` selects `sum`, and `E-DIM-ADD-*` selects the `+` or `-` that
joins the incompatible terms.
`SampleGenerationInfo` exposes exact binding and `sample <method>` expression
spans, while each `SampleDistributionInfo` exposes its attached option key and
value spans. Missing sampling options can therefore select the owning RHS, and
sampling semantic tokens plus Outline selections do not reconstruct names from
line text.

Simulation and algebraic/component-solve validation reuses the same inferred
declaration RHS plus `WithOptionInfo.value_span`. An unknown `simulate` or
`solve` target selects the target-name subslice, a missing required option
selects the owning RHS because no value exists, and a malformed supplied input,
parameter, timestep, duration, tolerance, solver, or solver-control option
selects its exact value. The compiler therefore owns these Problems ranges
before the LSP converts them to UTF-16.

`CommandStyleInfo` exposes the complete command expression, its target, and
each clause name/value span. `AssertInfo` likewise retains the left operand,
operator, right operand, and optional tolerance spans. Validation diagnostics,
command semantic roles, and command/assert Outline selections can therefore
use parser-owned occurrences even when the same text appears in a binding,
string, or trailing comment.

Outline containers also retain their own parser anchors. `ArgsBlockInfo.span`
and `AssertInfo.span` identify their exact keywords. `TestInfo.span` and
`GoldenInfo.span` identify unquoted declared string content, while
`ExpectationSuiteInfo.span` and `ExpectationInfo.span` identify the `expect`
keyword and first subject token. Assertions select `operator_span` first and
fall back through operand spans to the `assert` keyword.
`TableTransformInfo.binding_span` and `NetRequestInfo.binding_span` preserve the
exact result binding rather than only its line number.

`MlInfo` separately retains exact binding, source-model/table, and
prediction-input spans. It also exposes the complete ML expression, unified
inline/attached `with` arguments through `MlArgumentInfo.key_span` and
`value_span`, and each feature path through `MlFeatureInfo.span`. Trailing line
comments are not part of an option value. Alias and ML source/argument
diagnostics and editor roles can therefore target the resolved operand or
individual malformed value without repainting a same-spelled binding or grammar
word elsewhere in the expression or file.

`UncertaintyInfo` likewise preserves the exact binding and expression spans,
optional source span, positional values, and named argument key/value spans.
Its top-level argument splitter respects nested calls, lists, objects, strings,
and escapes. `E-UNC-SOURCE-*` and `E-UNC-ARGS-*` diagnostics and uncertainty
semantic roles therefore use the constructor occurrence that owns the value;
direct comparison diagnostics select the uncertain operand, percentile unit
mismatches select the incompatible threshold, and invalid probability forms
select the complete `probability(...)` expression.

## Semantic Analysis

`semantic.rs` builds the `CheckReport`. The report carries diagnostics plus a
`semantic_program` that records the facts reused by runtime and editor tooling:

```text
semantic_program.typed_bindings
semantic_program.expected_types
semantic_program.type_infos
semantic_program.unit_derivations
semantic_program.hover_hints
semantic_program.schemas
semantic_program.state_space_type_blocks / state_space_vectors / linear_operators
semantic_program.table_transforms
semantic_program.net_requests / net_downloads / cache_records
semantic_program.sample/case generations / render templates / uncertainty / model / db reads
semantic_program.reports / plots / writes / side-effect records / db records
```

`TypedBinding.is_top_level` records whether a typed value belongs to the
file/module value scope; scoped system, schema, operator, vector, and `where`
bindings remain available for tooling without being exposed as root expressions.

Table transforms and bound HTTP requests expose their declaration binding spans
alongside normalized operation metadata. Args, test/assert/golden, and
expectation records expose declaration/container spans. These fields are public
semantic contracts consumed directly by Outline generation and regression
tests, not editor-only reconstructed offsets.

Supported deprecated or invalid syntax, such as `:=`, `struct Args`, and
`script` execution roots, is reported through source-ranged diagnostics instead
of being silently accepted. Quantity and unit checks also produce source-ranged
diagnostics and review metadata.

Top-level state-space type blocks preserve separate keyword and declaration-name
spans. Each member preserves its name, normalized type, exact source type span,
optional unit, and exact unit span. The public semantic model retains those
ranges so solver lowering, review JSON, semantic highlighting, hover,
completion, outline, and navigation consume one declaration identity.

System and component typed declarations retain separate name, type, optional
unit, and optional expression spans. `operator Name:` also separates the
`operator` keyword anchor from `Name`. `SystemVariableInfo`,
`StateSpaceVectorInfo`, and `LinearOperatorInfo` expose those ranges so editor
tokens and diagnostics do not reconstruct typed declarations from line text.

Imports retain a keyword anchor plus an exact target span; quoted file imports
exclude the quote delimiters from the target range. Const declarations retain
separate name, type, optional explicit-unit, and expression spans. In the public
semantic model, `ImportInfo.span` identifies the import target and
`ConstInfo.span` identifies the const name. `ConstInfo.unit` is the optional
source annotation, while `display_unit` is the resolved display unit. Import
and const diagnostics, semantic tokens, and outline selection ranges consume
these fields directly.

The strict scalar-suffix recheck may preserve unchanged imported state-space and
component-template prefixes only after replaying their source registry. State-space
members, vector layouts, operator endpoints, shapes, compatibility, canonical
matrices, and parallel editor records must reproduce from semantic declarations.
Component headers, parameters, inputs, ports, local expressions/equations,
sequential signal contracts, root-owned instances/connections, and assembly graphs
must also reproduce. Its suffix environment includes eligible root scalar
declarations and importable constants, never system- or component-local variables.
System-scoped component instances/connections from imported modules are not imported.
Any ownership or derived-metadata mismatch returns to a full check.

Function declarations retain exact parameter name/type/optional-unit spans and
return type/optional-unit spans. `FunctionParamInfo.unit` and
`FunctionInfo.return_unit` identify explicit source annotations, while their
`display_unit` fields remain resolved values. Unknown signature-type diagnostics,
quantity/unit semantic tokens, and function/parameter/local outline selections
consume the same parser-owned ranges.

Block `return expression` and inline `fn ... = expression` forms retain an exact
expression span in `ReturnDecl` and `FunctionInfo.return_expression_span`.
Duplicate, unresolved, dimension-mismatched, and side-effecting return
diagnostics underline that expression. A missing-return diagnostic instead uses
the function name because no expression exists.

Domain declarations retain generic kind/name spans, variable type/unit spans,
and conservation keyword/expression spans. Component metadata retains parameter
type/unit/default spans, local expression and equation-side spans, constructor
argument values, and both connect endpoints. System equations likewise retain
their relation and left/right source ranges. Domain contract and quantity errors,
connect endpoint/compatibility errors, component parameter/equation/boundary and
delay/predictor/external behavior errors, and physical-equation diagnostics all
consume these ranges. The same metadata drives quantity/unit and endpoint
semantic colors plus source-owned domain, component, equation, and connection
Outline selections; synthesized residual metadata is not presented as a source
declaration.

Fast bindings whose RHS starts with `promote csv`, `promote json`, `promote
json records`, or `promote toml` retain a token-parsed `PromotionDecl`. It owns
the promotion/format/`records`/`as` tokens plus exact source, source-path
segment, and schema-name spans. `CsvPromotion` and `ConfigPromotion` expose the
binding, expression, promotion keyword, format, optional `records`, `as`, source,
and schema spans as optional public metadata so
runtime-synthesized table records can remain source-less. Schema lookup errors
select the schema name; source, missing-column, Args-source, and config
validation errors select the source operand. A failed CSV/JSON/config read does
not also infer missing columns or fields from an empty payload. Promotion
semantic roles and Outline entries consume these same ranges rather than
searching for repeated text on the source line.

Class declarations retain exact field default, validation expression, method
return type/unit, and method expression spans. Class object fields retain their
value expression spans. Every `E-CLASS-*` diagnostic consumes one of these
parser-owned ranges or an existing class/object/call name span: validation
failures prefer the explicit object-field value involved in the rule, missing
fields select the object name, and method-call errors select the receiver,
method, or supplied argument that failed. Trailing comments are outside these
ranges. The LSP reuses the same metadata for method quantity/unit colors and
class/object Outline selections.

## Editor Payload

`eng_lsp` maps the same `CheckReport` into editor-facing data:

```text
diagnostics
validation records with source origin
hover items
completion items
semantic tokens
document/workspace symbols
folding ranges
formatting and code actions
generated editor metadata
```

The VS Code extension and native IDE consume this shared payload. The generated
TextMate grammar, completion catalog, semantic legend, and syntax catalog are
rebuilt from compiler/LSP metadata so first-paint highlighting and live semantic
highlighting stay aligned. Snapshot hover and review-span JSON exposes
`source_origin` as `root` or `import`; consumers must not project an imported
line/column pair onto the checked root buffer. `CheckReport::source_files`
retains the root/import source registry behind compiler spans, and
`source_path_for_id` resolves that ownership without exposing the internal
numeric ID in JSON. Import-owned hover and validation spans project a portable
`source_path` for editor navigation.

## Boundaries

The frontend is not a claim that every planned language surface is implemented.
Unsupported syntax should either remain out of public examples or produce a
clear diagnostic with a source range. Public docs should describe the executable
compiler/runtime behavior that exists today and keep broader plans in current or
internal planning documents.
