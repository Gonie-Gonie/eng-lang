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
```

The CLI, LSP, VS Code extension, and native IDE all rely on these spans so that
Problems ranges, underlines, hover locations, and highlight inspection rows point
at the same source text.

`Diagnostic::source_span` is optional while older checks are migrated. When it
is present, it is authoritative: review JSON uses its starting column and the
LSP converts its byte column and byte length to UTF-16 editor coordinates.
Checks without a compiler-owned range still use the documented LSP range
inference path. `with` options preserve separate whole-option, key, and value
spans so option diagnostics do not need to search the source line by wording.
Line starts are counted from the original bytes, including two-byte CRLF line
endings.

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

`WriteDecl.expression_span` identifies the exact source expression after the
selected write path/format syntax, and `WriteInfo.expression_span` carries it
through the public semantic model. Editor projection can therefore classify a
simple write-source identifier from its compiler role even when its spelling is
also a workflow keyword, without changing a real keyword occurrence elsewhere.

`FastBinding.expression_span` identifies the complete source RHS before later
semantic normalization. Successful inferred declarations retain that range.
`SampleGenerationInfo` exposes exact binding and `sample <method>` expression
spans, while each `SampleDistributionInfo` exposes its attached option key and
value spans. Missing sampling options can therefore select the owning RHS, and
sampling semantic tokens plus Outline selections do not reconstruct names from
line text.

`CommandStyleInfo` exposes the complete command expression, its target, and
each clause name/value span. `AssertInfo` likewise retains the left operand,
operator, right operand, and optional tolerance spans. Validation diagnostics,
command semantic roles, and command/assert Outline selections can therefore
use parser-owned occurrences even when the same text appears in a binding,
string, or trailing comment.

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
semantic_program.sample/case generations / render templates / uncertainty / model / db records
semantic_program.reports / plots / writes / side-effect records
```

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

## Editor Payload

`eng_lsp` maps the same `CheckReport` into editor-facing data:

```text
diagnostics
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
highlighting stay aligned.

## Boundaries

The frontend is not a claim that every planned language surface is implemented.
Unsupported syntax should either remain out of public examples or produce a
clear diagnostic with a source range. Public docs should describe the executable
compiler/runtime behavior that exists today and keep broader plans in current or
internal planning documents.
