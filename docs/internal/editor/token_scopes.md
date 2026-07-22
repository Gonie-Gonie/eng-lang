# EngLang Editor Token Scopes

This note is the maintainer-facing contract for EngLang highlighting. It
documents the current TextMate fallback scopes, semantic token modifiers, and
the files that keep VS Code and the native IDE aligned.

## Source Of Truth

| Surface | Source |
| --- | --- |
| VS Code language configuration | `tools/vscode-englang/language-configuration.json` |
| TextMate fallback grammar | `tools/vscode-englang/syntaxes/eng.tmLanguage.source.json` |
| Generated TextMate grammar | `tools/vscode-englang/syntaxes/eng.tmLanguage.json` |
| Semantic token legend | `eng-lsp --editor-metadata` |
| Generated editor metadata | `tools/vscode-englang/generated/editor/englang-editor-metadata.json` |
| Generated syntax catalog | `tools/vscode-englang/generated/editor/englang-syntax.json` |
| Workflow status literal catalog | `syntax_catalog.workflow_status_literals` from `eng-lsp --editor-metadata` |
| Workflow builtin role catalog | `syntax_catalog.workflow_builtin_groups` from `eng-lsp --editor-metadata` |
| Native IDE lexical fallback catalog | `eng-lsp --editor-metadata` via `ide_bootstrap.syntaxCatalog` |
| VS Code semantic fallback scopes | `tools/vscode-englang/package.json` |
| Optional VS Code color themes | `tools/vscode-englang/themes/englang-dark-color-theme.json` and `tools/vscode-englang/themes/englang-light-color-theme.json` |
| Grammar smoke fixtures | `tools/vscode-englang/test/grammar-fixtures/*.eng` |
| Grammar smoke expectations | `tools/vscode-englang/test/expected/grammar_tokens.json` |
| TextMate/semantic parity audit | `tools/vscode-englang/test/textmateExampleCoverage.test.js` with `build/editor-tests/semantic_tokens/textmate_semantic_snapshots.json` |

Edit the source grammar, not the generated grammar. The source grammar may use
`{{...}}` placeholders for compiler-owned keyword, workflow helper, option,
type, and unit lists; `build-grammar.ps1` expands them from generated editor
metadata, including `syntax_catalog.workflow_status_literals` for `status ==`, `status !=`, and `status =` values. After grammar changes run:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat vscode-build-grammar
.\dev.bat vscode-grammar-test
.\dev.bat vscode-test
```

Role-specific builtin first-paint lists are compiler-owned under
`syntax_catalog.workflow_builtin_groups`. The grammar generator and native IDE
lexical fallback consume those groups directly; do not add parallel hardcoded
model, uncertainty, TimeSeries, solver, path, temporal, or boundary lists.

Compiler-resolved declaration overlays carry parser-owned name ranges through
`TypedBinding`, `HoverHint`, function/parameter/local symbol metadata,
schema/system/domain/component/class container metadata, and nested schema,
system, component, class, args, and object symbols. The LSP validates and consumes
those exact spans. Every ranged helper first verifies that the span belongs to
the checked root source; an import-owned span is authoritative for its own file
and must not trigger current-line search in the root buffer. Imported symbol
definitions remain in the resolver so root references can receive their
resolved role, but imported declaration/definition modifiers are not projected
onto those references. Function-scope reference scans must also skip
already-classified declaration ranges so different token types never overlap.
Domain variables, component ports, and class methods keep separate keyword anchors
and lexer-owned name spans; editor declarations consume the name spans.
Schema, class, and args type references, schema/class units, port domain references, and
class-object class/copy-source references retain parser-owned source ranges in
semantic metadata. Normalized type names are resolved only inside their source
range, generic types emit one token per identifier instead of an overlapping token
for the whole expression, and lexical unit fallback ignores declaration labels.
Component templates and system-local component instances are separate compiler
collections. Editor symbol passes consume both collections: template declarations
own their nested port/parameter/local tokens, while instance bindings own only the
instance declaration range and constructor references resolve back to the template
type.

After LSP semantic legend or completion changes run:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat ide-check
```

## TextMate Scope Naming

TextMate scopes should stay stable and broadly theme-compatible:

| Scope family | Use |
| --- | --- |
| `comment.line.*.englang` | Line comments (`#`, `//`) and documentation comments (`///`). |
| `string.quoted.double.englang` | Double-quoted string literals. |
| `constant.character.escape.englang` | Escape sequences such as `\n`, `\"`, and `\\` inside strings. |
| `keyword.control.*.englang` | Workflow, report, validation, solver, model, deprecated, side-effect, and external-boundary words, including `check coverage` clause words for validation-like review operations. |
| `keyword.operator*.englang` | Word and symbolic operators. |
| `punctuation.section.*.englang` | Block, bracket, and parenthesis delimiters. |
| `punctuation.separator.*.englang` | Separators such as commas and colons. |
| `punctuation.separator.parameter.englang` | Function and method parameter-list separators captured while declaration parameters are incomplete. |
| `punctuation.accessor.*.englang` | Accessor punctuation such as dots in paths or members. |
| `meta.block.header.englang` | Full non-validation block opener headers such as `args {`, `report {`, `where {`, and `on {`. |
| `meta.block.validation.englang` | Full validation block opener headers such as `constraints {` and `missing {`. |
| `storage.type.*.englang` | Captured block opener keywords and declaration-level markers. |
| `storage.type.function.englang` | `fn` and `method` keyword fallback coloring while declarations are incomplete. |
| `storage.type.test.englang` | `test` keyword fallback coloring while test declarations are incomplete. |
| `storage.type.block.englang` | `args`, `where`, `with`, and `on` block-opener fallback coloring while block headers are incomplete. |
| `storage.type.interface-member.englang` | `port`, `across`, and `through` interface-member keyword fallback coloring. |
| `storage.modifier.*.englang` | Modifiers such as schema indexes, constants, and system member roles. |
| `storage.modifier.schema.englang` | Schema `index` modifier fallback coloring from TextMate and semantic token mappings. |
| `storage.modifier.state.englang` | `state` member keyword fallback coloring aligned with state semantic colors. |
| `storage.modifier.input.englang` | `input` member keyword fallback coloring aligned with input semantic colors. |
| `storage.modifier.parameter.englang` | `parameter` member keyword fallback coloring aligned with parameter semantic colors. |
| `storage.modifier.output.englang` | `output` member keyword fallback coloring aligned with output semantic colors. |
| `storage.modifier.englang` | Generic modifier and constant keyword fallback coloring from semantic token mappings. |
| `entity.name.type.declaration.englang` | Full type-like declaration phrases such as `schema SensorData`. |
| `entity.name.type.englang` | Captured declaration names after `schema`, `system`, `domain`, `component`, and `class`, plus class names in object construction headers. |
| `meta.declaration.class-object.englang` | Full class object construction headers written as `object = Class {`. |
| `meta.declaration.class-object-copy.englang` | Full class object copy-with headers written as `object = source with {`. |
| `meta.declaration.function.englang` | Full `fn` and `method` declaration phrases. |
| `entity.name.function.englang` | Captured `fn` and `method` names. |
| `entity.name.function.call.englang` | User-defined function call names. |
| `entity.name.function.workflow-step.englang` | Workflow step names and compiler-backed workflow-step semantic fallbacks. |
| `entity.name.function.solver.englang` | Equation and operator declaration names used by solver blocks. |
| `meta.declaration.constant.englang` | Full `const` declaration phrases. |
| `meta.declaration.constant.typed.englang` | Typed `const name: Type [unit]` declaration phrases. |
| `variable.other.constant.englang` | Captured `const` names. |
| `variable.other.state.englang` | Captured `state` member names. |
| `variable.other.input.englang` | Captured `input` member names. |
| `variable.other.output.englang` | Captured `output` member names. |
| `variable.other.parameter.englang` | Captured system parameter names. |
| `meta.declaration.test.englang` | Full `test "..."` declaration phrases. |
| `entity.name.section.englang` | Captured quoted test names. |
| `support.type.englang` | Quantity/type names. |
| `meta.type.generic.englang` | Generic type expressions such as `Array[Record]`, `List[String]`, `Table[T]`, `Optional[DirectoryPath]`, and `TimeSeries[Time]`. |
| `meta.type.array-suffix.englang` | Array suffix type expressions such as `Bool[]` and `String[]`. |
| `variable.parameter.type.englang` | Generic type arguments inside bracketed type expressions. |
| `support.function.builtin.englang` | Error-tolerant fallback for incomplete or not-yet-classified builtin editor states; public helper fixtures should use a role-specific scope. |
| `support.function.validation.englang` | Validation and data-quality compatibility coloring for `fill_missing(...)`; this does not make it a public top-level call, and current authoring uses the `fill missing ...` workflow phrase. |
| `support.function.deprecated.englang` | Deprecated helper calls such as `select_first_row(...)`, `regression_table(...)`, and `train_regression(...)`; editor migration actions target their preferred forms. |
| `support.function.model.englang` | Preferred model-training and model-summary helper calls such as `regression(...)`, `train_test_split(...)`, `mlp(...)`, `evaluate(...)`, `model_card(...)`, and `leakage_lint(...)`. |
| `support.function.uncertain.englang` | Uncertainty helper calls such as `measured(...)`, `interval(...)`, `normal(...)`, `uniform(...)`, `distribution(...)`, `propagate(...)`, `ensemble(...)`, and `probability(...)`. |
| `support.function.timeseries.englang` | Native TimeSeries/statistic calls such as `integrate(...)`, `mean(...)`, `min(...)`, `max(...)`, `median(...)`, `std(...)`, `sum(...)`, `time_weighted_mean(...)`, `p90(...)`, and `duration_above(...)`; the latter also has a compact threshold-only selector inside `summarize`. |
| `support.function.external-boundary.englang` | External boundary constructors/checks such as `file(...)`, `dir(...)`, `url(...)`, `env(...)`, `secret env(...)`, and `exists(...)`. |
| `support.function.db.englang` | SQLite `.table(...)` method names inside native DB read/write phrases. |
| `support.function.workflow-step.englang` | Workflow-step helper calls such as `apply(...)` and step values such as `run_case`. |
| `support.function.solver.englang` | Solver-context calls: `der(...)` is an equation operator and `delay(...)` is a component behavior call, not a general top-level helper. |
| `support.function.path.englang` | Path helper calls such as `join(...)`, `parent(...)`, `stem(...)`, and `extension(...)`. |
| `support.function.temporal.englang` | Supported temporal helper calls such as `date(year, month, day)`. |
| `support.namespace.module.englang` | Module namespaces such as `eng.table`. |
| `constant.numeric*.englang` | Numeric literals and format precision fragments. |
| `constant.other.unit*.englang` | Unit literals. |
| `constant.language.englang` | Language constants and uncertainty/fallback words. |
| `variable.parameter.property.englang` | `args.*`, `with` options, and parameter-like property paths. |
| `meta.declaration.args-block.englang` | Full `args { ... }` execution-argument blocks. |
| `meta.declaration.argument.englang` | Argument field declaration fragments inside `args { ... }`. |
| `variable.parameter.argument.englang` | Captured execution argument names inside `args { ... }`. |
| `variable.parameter.function.englang` | Function and method parameter declarations plus compiler-catalog and function-style named call arguments such as `std=`, `algorithm=`, or `split=`. |
| `variable.language.self.englang` | `self` references inside class methods. |
| `variable.other.local.englang` | Runtime binding references inside workflow phrases. |
| `meta.declaration.parameter.englang` | Function and method parameter declaration fragments. |
| `meta.declaration.typed-binding.englang` | Typed value declarations written with `name: Type = ...`. |
| `meta.declaration.field.englang` | Field declarations written with `name:`. |
| `variable.other.property.englang` | Schema/class fields, component members, property paths, and object fields. |
| `meta.path.property.englang` | Full dotted property paths such as `samples.row_preview` for TextMate first-paint grouping. |
| `meta.path.public-member.englang` | Public workflow member paths such as `samples.row_preview` and `study.designs.row_preview`. |
| `meta.path.parameter.englang` | Full `args.*` paths for TextMate first-paint grouping. |
| `meta.path.parameter-public-member.englang` | Public workflow member paths reached through `args.*`, such as `args.designs.row_preview`. |
| `variable.other.member.englang` | Member segments after a dot in dotted property paths. |
| `variable.other.public-member.englang` | Compiler-catalog public workflow API member segments such as `response_source`, `row_preview`, DB `summary`, case count fields, `model.rmse`, and `predictions.output_column`. |
| `variable.parameter.property.member.englang` | Member segments after `args.` in dotted parameter paths. |
| `variable.other.definition.englang` | Runtime binding names written with `name = ...`. |
| `variable.other.model.englang` | Class object source names in copy-with headers. |
| `meta.workflow.*.englang` | Phrase scopes for multi-token workflow operations. |
| `meta.report.*.englang` | Report phrase scopes. |
| `meta.quantity.literal.englang` | Unit-bearing numeric expressions. |
| `meta.interpolation.englang` | String interpolation bodies. |
| `punctuation.separator.format.englang` | Format separator inside string interpolation (`:`). |
| `constant.numeric.format.englang` | Numeric precision fragments inside string interpolation, such as `.2`. |
| `constant.other.unit.format.englang` | Unit display fragments inside string interpolation format specs. |
| `invalid.deprecated.englang` | High-risk fallback mapping. |
| `markup.warning.englang` | Medium-risk fallback mapping. |

Command-style workflow and review verbs such as `sample`, `filter`, `derive`,
`require_one`, `integrate`, and `mean` use `keyword.control.*.englang`. Model
workflow phrases such as `predict ... using ...` and `train regression ...`
use `keyword.control.model.englang`; current model helper calls such as
`regression(...)`, `train_test_split(...)`, `mlp(...)`,
`evaluate(...)`, `model_card(...)`, and `leakage_lint(...)` use
`support.function.model.englang`. Compatibility-only `regression_table(...)`,
`train_regression(...)`, and `ann(...)` use `support.function.deprecated.englang` while
retaining the model semantic modifier.

Uncertainty helper calls such as `measured(...)`, `interval(...)`, `normal(...)`,
`uniform(...)`, `distribution(...)`, `propagate(...)`, `ensemble(...)`, and
`probability(...)` use `support.function.uncertain.englang`. TimeSeries/statistic
helper calls such as `integrate(...)`, `mean(...)`, `min(...)`, `max(...)`,
`median(...)`, `std(...)`, `sum(...)`, `time_weighted_mean(...)`,
`duration_above(...)`, `p90(...)`, and `p95(...)` use
`support.function.timeseries.englang`; arbitrary `pNN(...)` names are not
treated as implemented percentile helpers. `fill_missing(...)` uses
`support.function.validation.englang` while current authoring uses the
`fill missing ...` workflow phrase; `select_first_row(...)` uses
`support.function.deprecated.englang`.

Canonical uncertainty named arguments use
`variable.parameter.property.englang` on first paint and the `uncertain`
semantic modifier after analysis. Compatibility-only `sigma`, `uncertainty`,
`error`, `n`, `min`, `max`, `mu`, `distribution`, `gain`, and
`bias` keys use `keyword.control.deprecated.englang` first paint and
`property + uncertain + deprecated` semantic tokens.

External boundary constructors/checks such as `file(...)`, `dir(...)`,
`url(...)`, `env(...)`, `secret env(...)`, and `exists(...)` use
`support.function.external-boundary.englang`; path helpers such as `join(...)`,
`parent(...)`, `stem(...)`, and `extension(...)` use
`support.function.path.englang`; `date(...)` uses
`support.function.temporal.englang`; workflow-step helpers such as `apply(...)`
and `run_case` use `support.function.workflow-step.englang`; solver helpers such
as `der(...)` and `delay(...)` use `support.function.solver.englang`.
`url(...)` and non-secret `env(...)` are public typed value calls; the latter
accepts an optional fallback and records environment provenance. `secret
env(...)` remains the redacted credential form.
TimeSeries quality verbs such as `fill`, `align`, and `resample` use
validation-colored fallback scopes to match their phrase scopes. String
interpolation and the native IDE lexical first paint consume the same
compiler-owned builtin role groups before falling back to generic builtin
coloring. `support.function.builtin.englang` remains only as an error-tolerant
fallback for incomplete editor states, not as evidence that an additional
public call-style API is implemented.

These color roles are context classifications, not top-level API claims.
`duration_above(...)` is both a native value call and a compact
`summarize` statistic selector, `delay(...)` is
component behavior syntax, and `fill_missing(...)` is compatibility-colored
while public authoring uses `fill missing ...`.

`#members` must appear before generic `args.*` and dotted-path fallbacks inside expression contexts so TextMate first-paint tokenization can split roots, dots, and member segments before broad property regexes match the whole path. Grammar smoke also requires begin/end workflow phrase scopes to include `#members`, so operand-oriented phrases cannot regress to uncolored dotted paths.

The dedicated `withOptions` rule begins at `^\s*with`. Its line anchor
lets the top-level include order select it before the generic block rule, so
option keys and report `unit <axis> =` rows receive their intended scope.
The standalone `with` keyword always carries `workflowStep` semantically,
in addition to any command-domain modifier, because first-paint parsing cannot
recover the preceding owner command after its line rule has ended.

Connection phrases have a dedicated `meta.solver.connect.englang` rule.
Both `connect` and the `to` clause word use
`keyword.control.solver.englang`; the generic workflow `to` scope must not
win inside `connect A to B`.

Prefer adding a phrase-level `meta.workflow.*.englang` scope when a native
workflow operation is more readable as a single action than as unrelated
keywords. Examples include `sample lhs`, `predict model using`, `read json`,
`open sqlite`, and `write ... to db.table(...)`.

For `promote csv/json/toml` and `promote json records`, TextMate first-paint
scopes keep the phrase workflow-colored and include member-aware source-path
fallbacks, while LSP semantic tokens add `workflowStep` plus `external` to
source operands such as `file(...)`, `args.input`, `payload`, and
`payload.records`. The LSP constrains binding, source, and schema roles to the
token-parsed promotion spans. Repeated names on one line therefore remain a
declaration variable, source variable/property path, and schema class at their
own occurrences instead of being repainted by first-text search.

I/O and external-boundary phrases such as `read json`, `write text`,
`write json`, `write standard_text`, `export summary to csv`, `download`,
`http ...`, `render template`, and file operations also keep `#members`
before broad `args.*` and dotted-path fallbacks so source and target operands
split consistently before semantic highlighting arrives.

DB/table phrases such as `open sqlite`, `read sqlite <db>.table(...)`,
`write <table> to <db>.table(...)`, and `select <table> columns ...` use the
same member-aware fallback ordering so nested DB/table receivers and selected
source tables split before broad property scopes. The `table` method itself
uses `support.function.db.englang` so first-paint DB coloring agrees with the
`method.db` semantic role.
DB read/write phrase rules match the full receiver plus `.table` path before
the generic member rule; matching only the final word cannot win against a dotted
path pattern that starts earlier on the line.

Table and case operation phrases such as `filter`, `derive`, `sort`, `join`,
`require_one`, `materialize cases`, and `collect results` use phrase-level
member-aware fallbacks so dotted table and case operands stay split on first
paint.

Predict and train phrases such as `predict <model> using <table>` and
`train regression <table>` keep model-colored first-paint keywords while routing
dotted model and table operands through member-aware fallbacks. Apply phrases such
as `apply <step> over <table>` and `apply(..., over=...)` keep workflow-step
coloring for step/case orchestration and use the same member-aware operand split.

Integrate/statistical calls such as `integrate(<series>, over=<axis>)` and
`mean(<series>, axis=<axis>)`, plus summary export fields like `<value> as
<unit> with <format>`, use member-aware fallbacks so dotted series, axis, and
summary operands split before broad property scopes.

Grammar smoke tests now enforce that workflow scopes using property fallback
coloring include `#members` first; `status` option/condition scopes are the
only allowed exception because they color fixed workflow-state keys and literals.

Model call phrases such as `train_test_split(...)`, `regression_table(...)`,
`train_regression(...)`, `evaluate(...)`, `model_card(...)`, and
`leakage_lint(...)` also include
member-aware fallbacks so nested model, split, table, and feature operands stay
split before broad property scopes.

Uncertainty distribution calls such as `measured(...)`, `interval(...)`,
`propagate(...)`, `ensemble(...)`, and `probability(...)` also keep `#members`
ahead of broad dotted-path fallbacks for nested uncertainty operands.

TimeSeries quality phrases such as `check coverage`, `fill missing`, `align`,
and `resample` also include member-aware source-path fallbacks so
`measured.T_zone`, `simulated.T_zone`, and `args.measured_zone` split into
receiver, dot, and member scopes before semantic highlighting arrives.
Command-style `integrate <series> over <axis>` and
`mean/max/min <series> over <axis>` phrases use the same member-aware fallback
while call-style `integrate(...)` and `mean(...)` remain under their
function-call phrase scopes.

Current workflow phrase scopes:

| Scope | Phrase |
| --- | --- |
| `meta.workflow.apply-call.englang` | `apply(<step>, over=<table>)` |
| `meta.workflow.apply-step.englang` | `apply <step> over <table>` |
| `meta.workflow.align-series.englang` | `align <series> with <series>` or `align <series> to <series>` |
| `meta.workflow.check-coverage.englang` | `check coverage <series>` |
| `meta.workflow.collect-results.englang` | `collect results <table>` |
| `meta.workflow.db-read.englang` | `read sqlite <db>.table("<name>") as <schema>` |
| `meta.workflow.db-write.englang` | `write <table> to <db>.table("<name>")` |
| `meta.workflow.derive-column.englang` | `derive <table> column ...` |
| `meta.workflow.distribution-call.englang` | `uniform(...)`, `normal(...)`, `distribution(...)`, `measured(...)`, and `interval(...)` value distributions. |
| `meta.workflow.download-to.englang` | `download ... to ...` |
| `meta.workflow.export-summary-csv.englang` | `export summary to csv <target>` |
| `meta.workflow.file-operation.englang` | Top-level `copy <source> to <destination>`, `move <source> to <destination>`, `delete <target>`, and `mkdir <target>` file operation statements. |
| `meta.workflow.fill-missing.englang` | `fill missing <series>` |
| `meta.workflow.filter-table.englang` | `filter <table>` |
| `meta.workflow.http-request.englang` | `http get <target>`, `http post <target>`, and other HTTP request phrases. |
| `meta.workflow.integrate-call.englang` | `integrate(<series>, over=<axis>)` |
| `meta.workflow.integrate-series.englang` | `integrate <series> over <axis>` |
| `meta.workflow.join-table.englang` | `join <left> with <right>` |
| `meta.workflow.log-message.englang` | `log <level> ...` structured runtime message lines; includes member-aware first-paint fallbacks for dotted message operands. |
| `meta.workflow.materialize-cases.englang` | `materialize cases <table>` |
| `meta.workflow.model-summary-call.englang` | `evaluate(<model>[, split=...])`, `model_card(<model>)`, and related model summary calls. |
| `meta.workflow.model-train-call.englang` | `train_test_split(...)`, `regression(...)`, and `mlp(...)` model-training calls. |
| `meta.workflow.legacy-model-train.englang` | Compatibility-only `regression_table(...)`, `train_regression(...)`, and `ann(...)` calls with deprecated function-name coloring and model-aware arguments. |
| `meta.workflow.train-regression.englang` | `train regression <table>`, `train regression from <table>`, and `train regression on <table>` table-model training phrases. |
| `meta.workflow.open-sqlite.englang` | `open sqlite <source>` |
| `meta.workflow.option-map.englang` | `query = { ... }`, `headers = { ... }`, and `values = { ... }` option maps. |
| `meta.workflow.with-block.englang` | `with { ... }` option blocks scoped separately from top-level bindings. |
| `meta.workflow.predict-model.englang` | `predict <model> using <table>` |
| `meta.workflow.print-message.englang` | `print ...` runtime message lines; includes member-aware first-paint fallbacks for dotted message operands. |
| `meta.workflow.plot-distribution.englang` | `plot distribution(<distribution>)`; includes member-aware first-paint fallbacks for dotted distribution operands. |
| `meta.workflow.plot-command.englang` | Fallback for multi-series `plot <a> and <b> over <axis>` and named plot functions such as `plot histogram(...)`, `plot parity(...)`, and `plot residuals(...)`; includes member-aware first-paint fallbacks for dotted report operands. |
| `meta.workflow.plot-series.englang` | `plot <series> over <axis>`; includes member-aware first-paint fallbacks for dotted series and axis operands. |
| `meta.workflow.promote-csv.englang` | `promote csv <source> as <schema>` |
| `meta.workflow.promote-json.englang` | `promote json <source> as <schema>` |
| `meta.workflow.promote-json-records.englang` | `promote json records <source> as <schema>` |
| `meta.workflow.promote-toml.englang` | `promote toml <source> as <schema>` |
| `meta.workflow.read-structured.englang` | `read json <source>`, `read toml <source>`, and `read text <source>` raw string reads; phrase keywords use external-boundary coloring because the source can be a file, response body, or other boundary value. Use `promote csv <source> as <schema>` for CSV tables. |
| `meta.workflow.require-one.englang` | `require_one <table>` |
| `meta.workflow.resample-series.englang` | `resample <series> to <series>`, `resample <series> with <series>`, or `resample <series> by <duration>` |
| `meta.workflow.render-template.englang` | `render template <source>` and `render template <source> to <output>` |
| `meta.workflow.return-statement.englang` | `return <value>` function return lines. |
| `meta.workflow.run-command.englang` | `run command ...`; includes member-aware first-paint fallbacks for command operands such as `args.simulator`. |
| `meta.workflow.sample-method.englang` | `sample lhs`, `sample grid`, `sample random`, and related sample methods. |
| `meta.workflow.select-columns.englang` | `select <table> column <column>` or `select <table> columns <columns>` |
| `meta.workflow.show-report.englang` | `show <value>` and optional report display suffixes; includes member-aware first-paint fallbacks for dotted report values. |
| `meta.workflow.sort-table.englang` | `sort <table> by <column> [asc|desc]` |
| `meta.workflow.stat-axis-call.englang` | `mean(<series>, axis=<axis>)`, `max(<series>, axis=<axis>)`, and related axis statistic calls. |
| `meta.workflow.stat-series.englang` | `mean <series> over <axis>`, `max <series> over <axis>`, and related command-style statistic phrases. |
| `meta.workflow.rmse-comparison.englang` | Compatibility `rmse <measured-series> vs <simulated-series>` command phrases; includes member-aware first-paint fallbacks for dotted operands. Preferred `rmse(left, right)` calls use the validation builtin scope and semantic operand roles. |
| `meta.workflow.status-condition.englang` | `status == passed` and related `on { ... }` status checks; the literal list is generated from `syntax_catalog.workflow_status_literals`. |
| `meta.workflow.status-option.englang` | `status = planned` and related workflow status option values inside `with { ... }`; the literal list is generated from `syntax_catalog.workflow_status_literals`. |
| `meta.workflow.summary-field.englang` | `<value> as <unit> with "<format>"` summary CSV fields. |
| `meta.workflow.summarize-series.englang` | `summarize <series> by [...]`; includes member-aware first-paint fallbacks for dotted series operands. |
| `meta.workflow.validation.englang` | `validate ...`, `assert ...`, and `golden ... matches ...` validation lines. |
| `meta.workflow.write-json.englang` | `write json <target>, <value>` |
| `meta.workflow.write-standard-text.englang` | `write standard_text <table>` with an output option or `to <target>`. |
| `meta.workflow.write-text.englang` | `write text <target>, <value>` |

## Semantic Token Legend

The current token types are the standard VS Code-compatible set:

```text
namespace, type, class, interface, parameter, variable, property, function,
method, keyword, modifier, string, number, operator, comment
```

The current semantic modifiers are:

```text
declaration, definition, readonly, local, imported, defaultLibrary, deprecated, documentation,
unit, quantity, axis, timeseries, uncertain, sideEffect, external,
validation, report, solver, planned, internal, riskHigh, riskMedium, state,
input, output, model, db, cache, workflowStep, path, temporal
```

The LSP modifier stream is limited to 31 entries so every modifier fits the
protocol's non-negative integer bitset. Function definitions may carry both
`declaration` and `definition`; the language does not emit a separate
static/class-level semantic role. Keep this list at or below 31 entries and use
token types or an existing role when a new distinction does not need its own
independently themeable bit.

Modifier meanings:

| Modifier | Meaning |
| --- | --- |
| `unit` | Unit symbols or unit-typed values. |
| `quantity` | Quantity types and quantity-bearing values. |
| `axis` | Time axes and aligned axis metadata. |
| `timeseries` | TimeSeries values and accessors. |
| `uncertain` | Measured, interval, ensemble, fallback, or propagated uncertainty values. |
| `sideEffect` | Filesystem or other declared side effects; risk-bearing side effects also carry `riskHigh` or `riskMedium`. |
| `external` | File, process, HTTP, DB, or other external boundaries. |
| `validation` | Validation, assertion, golden, and coverage operations. |
| `report` | Report, plot, summary, and rendered artifact operations. |
| `solver` | Solver, equation, conservation, simulation, and component-solve terms. |
| `planned` | Planned module symbols or not-yet-supported workflow nodes. |
| `internal` | Internal runtime/package implementation symbols. |
| `riskHigh` | High-risk review metadata. |
| `riskMedium` | Medium-risk review metadata. |
| `state` | State declarations and state-like workflow data. |
| `input` | Input declarations, parameters, and input-like workflow data. |
| `output` | Output declarations and output-like workflow data. |
| `model` | Model training, evaluation, prediction, and model-card artifacts. |
| `db` | SQLite connections, table writes, and DB boundary records. |
| `cache` | Cache declarations, keys, and cache-backed workflow records. |
| `workflowStep` | Case orchestration, sampling, prediction, and workflow-step symbols. |
| `path` | Path construction and path inspection helper calls. |
| `temporal` | Supported calendar date construction helper calls. |

Core semantic role expectations:

| Source role | Semantic token |
| --- | --- |
| `use`/`import` keyword | `keyword` with `declaration` and `imported`. |
| `use`/`import` namespace | `namespace` with `declaration` and `imported`. |
| `const` name | `variable` with `declaration` and `readonly`. |
| `const` declared type | `type` with the compiler-resolved quantity/type modifiers. |
| `const` explicit unit | `type` with `unit`. |
| Schema name | `class` with `declaration`. |
| Schema/class/component field | `property` with `declaration`. |
| `args` field | `parameter` with `declaration`. |
| Function parameter | `parameter` with `declaration`. |
| Function parameter declared type | `type` with the compiler-resolved quantity/type modifiers. |
| Function parameter explicit unit | `type` with `unit`. |
| Function return declared type | `type` with the compiler-resolved quantity/type modifiers. |
| Function return explicit unit | `type` with `unit`. |
| System state declaration | `variable` with `declaration` and `state`. |
| System input declaration | `variable` with `declaration` and `input`. |
| System parameter declaration | `parameter` with `declaration` and `readonly`. |
| System output declaration | `variable` with `declaration` and `output`. |
| State-space vector type block | `class` with `declaration`, `solver`, and role-specific `state`, `input`, or `output`. |
| State-space type member | `property` with `declaration`, `solver`, and the block's role modifier. |
| Function-local binding | `variable` with `local`. |
| Deprecated `script`/`struct` keyword | `keyword` with `deprecated`. |
| Deprecated `script`/`struct` declaration name | `class` with `declaration` and `deprecated`. |
| Bundled stdlib domain namespace | `namespace` with `defaultLibrary` and `internal`. |
| Supported/native `eng.*` module import | `namespace` with `declaration`, `imported`, and `defaultLibrary`. |
| Planned `eng.*` module import | `namespace` with `declaration`, `imported`, `defaultLibrary`, and `planned`. |
| Internal `eng.*` module import | `namespace` with `declaration`, `imported`, `defaultLibrary`, and `internal`. |
| String interpolation expression | `variable`/`parameter` plus `property` path segments inside `{...}` fields; format precision digits use `number`, and format units use `type` with `unit`. |

`planned` is currently emitted for source-visible planned stdlib module imports
such as `use eng.building`; internal stdlib imports such as `use eng.system`
carry `internal`. Native imports such as `use eng.stats` do not carry either
status modifier.

Import namespace tokens, const names, const types, and explicit const units use
parser-owned spans. The outline uses the same import-target and const-name
selection ranges, and compiler diagnostics use the import target or const
expression span. This avoids same-line text search when a const name is repeated
in its initializer and keeps UTF-16 ranges correct for non-BMP import paths.
An unquoted dotted import such as `eng.stats` is one atomic `namespace` range;
review-risk metadata is merged into that existing range instead of adding a
shorter variable token over its first segment.

Function names, parameter names, parameter type/unit annotations, and return
type/unit annotations also use parser-owned spans. Function and parameter
outline selections use their exact name occurrences, while `E-FN-TYPE-001` and
`E-FN-TYPE-002` underline the corresponding return or parameter type instead of
the `fn` keyword.

Function return metadata also preserves the exact expression after a block
`return` keyword or inline `=`. Duplicate, unresolved, dimension-mismatch, and
return-side-effect diagnostics use that compiler range; missing-return
diagnostics select the function name.

Component assembly balance and algebraic-loop diagnostics select the first
source component name used as the assembly anchor. Unconnected-port diagnostics
select the exact port name, while unknown or invalid generic domains select the
complete port-domain reference. Port parsing ends at the final lexer token, so a
trailing `#` or `//` comment cannot extend either the domain value or its
Problems range. The fixture-corpus guard validates these compiler-owned ranges
before UTF-16 conversion.

Domain generic kinds use `type.model`, their named parameters use
`parameter.declaration.model`, and domain/component quantity and explicit unit
annotations use `type.quantity` and `type.unit` from parser-owned spans.
Resolved connect endpoints split each source range into a `variable.model.solver`
component and `property.model.solver` port. Domain contract/quantity, connect
endpoint/compatibility, component parameter/equation/boundary, behavior-call,
and physical-equation diagnostics select the failing declaration, endpoint,
call, argument, unit, or equation side. Domain, component, source equation, and
connection Outline selections reuse those ranges; generated residuals are not
shown as source children. A dedicated corpus guard requires every targeted
diagnostic in these families to retain a valid compiler-owned range.

Promotion schema lookup diagnostics select the exact schema token after `as`.
CSV/JSON source, required-column, Args-source, and config source/validation
diagnostics select the complete source operand. The fixture guard covers
`E-SCHEMA-PROMOTE-001`, `E-SCHEMA-CSV-*`, `E-SCHEMA-JSON-*`, and
`E-CONFIG-SOURCE-001`; unreadable sources no longer generate secondary
missing-column or missing-field diagnostics from empty inferred data.

HTTP request and download metadata preserve the exact URL operand span. URL
validation follows declared const/local aliases to a literal when possible and
uses the operand occurrence for `E-NET-INVALID-URL`; an unsupplied runtime
`args.*` reference remains unresolved rather than producing a false malformed-URL
diagnostic. Fast-binding and download operands end at the final lexer token, so
trailing comments do not extend their Problems or review ranges.

`where`/`with` opener tokens and `where` local declarations use their
parser-owned semantic spans. `with` option properties stay inside `key_span`;
model list values, enum-like values, and `file`/`dir`/`join` helpers stay inside
`value_span`. Inline options therefore cannot repaint matching text in an
earlier option or string, and `where` local plus `with` option outline selections
use the same UTF-16-safe ranges. A `unit <axis>` key is split only within its key
span so `unit` remains a keyword and the axis remains a property. Multiline
option values end at the final lexer token, excluding trailing `#` and `//`
comments; inline block delimiters likewise come from brace tokens rather than
brace characters inside strings or comments.

Print/log templates, CSV export sources and fields, write sources and targets,
DB read/write connection and table paths, file-operation operands, and process
bindings/commands use their public compiler-owned side-effect spans. Their
semantic roles are emitted only inside those ranges, including interpolation
expressions, requested units, dotted receivers, and quoted path contents.
Repeated text in a binding, string, or earlier operand cannot receive the later
role.

For a simple `write` source identifier, the lexical keyword fallback at
`WriteInfo.expression_span` is replaced by one variable token carrying the
binding role plus side-effect and DB context. The same spelling in a real
grammar position, such as `records` in `promote json records`, remains a
workflow keyword. DB reads and writes share parser-owned `DbTableTargetInfo`
ranges with native runtime execution rather than rediscovering a connection or
table from line text.

Simple inferred aliases use the preserved fast-binding expression span, and ML
model/table/input operands use their dedicated `MlInfo` spans. ML call options
also use structured key/value ranges, with each feature path retaining its own
span for diagnostics and semantic tokens. A binding named `model`, `records`,
`target`, or `algorithm` therefore keeps one variable role at its declaration
while the same spelling is classified independently in an ML operand or option.
Grammar uses such as `promote json records` remain keywords, and member uses such
as `payload.records` remain properties. Dotted ML operands and feature paths are
emitted per identifier: `args` is a parameter, another leading receiver is a
variable, and following segments are properties. Dotted command-style `apply`
targets use the same segmentation with the terminal segment classified as a
workflow-step function.

Sampling declarations use `SampleGenerationInfo.binding_span` for their variable
token and Outline selection. Distribution option names use
`SampleDistributionInfo.key_span`; their value spans remain available for
diagnostics and later editor projections. Missing `count` and missing parameter
diagnostics select the exact `sample <method>` expression rather than whichever
same-spelled word appears first on the line. The sampling corpus guard requires
every `E-SAMPLING-*` diagnostic to retain a valid compiler range.
Compatibility method diagnostics select only `uniform`, `latin_hypercube`,
or `latin-hypercube` after `sample`. Semantic tokens add `deprecated` only
in that method context; `uniform(...)` distribution calls remain uncertain
functions. TextMate first-paint uses
`meta.workflow.legacy-sample-method.englang` plus
`keyword.control.deprecated.englang` for the same three phrases.

Simulation and algebraic/component-solver diagnostics also consume source-owned
ranges. Unknown targets select only the target name, missing required options
select the owning `simulate`/`solve` RHS, and malformed supplied options select
their exact `WithOptionInfo.value_span`. The fixture-corpus guard requires every
observed `E-SIM-*` and `E-SOLVE-*` diagnostic to retain that compiler range.
Value quick fixes prefer the exact Problems range; when a single required
option is absent, the LSP inserts it into the attached `with` block or creates a
new block on the owner declaration.

Class field defaults, validation expressions, method return type/unit and return
expressions, and object-field values also retain compiler-owned spans. Every
`E-CLASS-*` diagnostic in the fixture corpus must select its failing name,
expression, or argument before UTF-16 conversion. Method return quantities and
units use those exact spans for semantic colors. Class, field, validation,
method, object, and explicit object-field Outline selections use their source
occurrences; inherited copy-with fields are omitted from the copy's Outline so
they cannot navigate back into the source object. Evaluated object validation
results stay in the validation payload instead of appearing as structural
Outline children that point back to the class rule.

Command-style targets and clause names/values use `CommandStyleInfo` spans for
semantic tokens. Command Outline entries select the target, while assertion
children select `AssertInfo.operator_span` with operand fallback. Direct
uncertainty comparisons underline the uncertain operand, percentile unit
mismatches underline the incompatible threshold, invalid probability forms
underline the complete call, and generic validation unit mismatches underline
the right operand. The validation corpus guard rejects a fallback range for
these diagnostic classes.

Uncertainty constructors use the same source-owned policy. The declaration uses
`UncertaintyInfo.binding_span`; `ensemble` and `propagate` sources use the exact
positional source span; named keys are `property.uncertain`; and `kind`,
`distribution`, and `method` values are `keyword.uncertain`. Dotted named values
are segmented into parameter/receiver/property roles only within their value
span. Repeating `method`, `kind`, or a source name elsewhere on the line cannot
repaint the declaration or another argument. `E-UNC-SOURCE-*` and
`E-UNC-ARGS-*` Problems ranges consume these compiler spans, including a whole
nested value such as `coalesce(5 kW, 6 kW)`.

Expression-type diagnostics also stay source-owned. Quantity ambiguity selects
the direct literal unit and ignores unit-looking string contents or function
arguments. HeatRate sum warnings select `sum`; dimensionless addition and
subtraction select the joining operator; function-call lookup/arity failures
select the called name; and argument type failures select the complete failing
argument. The fixture guard requires all observed diagnostics in these families
to retain a compiler range before UTF-16 conversion.

The remaining structural/workflow diagnostics follow the same policy.
Derivative duplicates select the repeated `der(...)` call, legacy
`select_first_row` warnings select the call name, missing join policy selects the
join expression, and run-case source/option/results errors select the source,
option key, or owner expression. Deprecated root/test/assert/golden diagnostics
select their declaration keyword or header, while implicit TimeSeries fill
warnings select the fill expression. Missing run-case results emit one Problems
entry. The global 153-file diagnostic audit rejects any range that would need
LSP text inference; its fallback count is zero.

Lexical numbers require identifier boundaries and consume a valid decimal
exponent as part of the same token. Names and literals such as `p95`, `rk4`,
`expected_sha256`, and `case_001` therefore do not acquire nested number colors.
Operator fallback excludes numeric exponents, generated hyphenated workflow
literals, known units, and exact compiler-owned unit spans, so `1.25e-3`,
`latin-hypercube`, and even a diagnosed composite unit such as `lb/s` remain
atomic. Lexical strings are split into the uncovered fragments around nested
interpolation parameters/properties, format precision numbers, display units,
and quoted import namespaces. This preserves each inner role at its exact UTF-16
range without asking clients to resolve an overlapping outer string token.
`lsp-check` rejects every overlapping semantic-token pair in all example and
grammar-fixture snapshots, including equal-type and whole-path/segment overlaps.

## VS Code Fallback Mapping

VS Code maps semantic tokens to TextMate fallback scopes in
`tools/vscode-englang/package.json` under `semanticTokenScopes`. Keep that map
in sync with the generated legend. `lsp-check` scans example and grammar-fixture
snapshots and fails when any observed semantic selector lacks a VS Code fallback
mapping. The VS Code highlight inspector also reports selectors that only receive
a base-token fallback so maintainers can distinguish fully mapped role colors
from generic fallback colors. Important pairings:

| Semantic selector | Fallback scope intent |
| --- | --- |
| `type` | Type names, generic type expressions, array suffix type expressions, and bracketed type arguments. |
| `class`, `interface`, `class.declaration`, `interface.declaration` | Declared schema, system, state-space vector type, component, domain, and interface-like names. |
| `class.defaultLibrary`, `interface.defaultLibrary` | Bundled type/domain names surfaced by the compiler. |
| `comment` | Ordinary (`#`, `//`) and documentation (`///`) line comments. |
| `comment.documentation` | Documentation comments (`///`) when semantic highlighting is available. |
| `function`, `function.declaration`, `function.definition`, `method`, `method.declaration` | User-defined function and method calls plus declaration names. |
| `function.report` | Report helper functions when emitted as semantic function tokens. |
| `keyword`, `keyword.declaration`, `keyword.local` | General workflow keywords, declaration keywords, schema-index modifier fallbacks, local keyword-like roles, and block-opener semantic fallbacks. |
| `modifier` | Modifier-like role tokens and schema indexes. |
| `namespace`, `namespace.declaration` | Imported or declared module namespaces. |
| `number` | Numeric literals. |
| `parameter`, `parameter.readonly` | Function parameters, args-like parameters, and read-only parameter roles. |
| `property`, `property.declaration`, `property.readonly` | Property paths, declared schema/class/system fields, and read-only property roles. |
| `variable`, `variable.local`, `variable.declaration`, `variable.defaultLibrary`, `variable.readonly`, `variable.deprecated` | Plain variables, local references, declared bindings, bundled value symbols, read-only constants, and deprecated variables. |
| `type.unit`, `property.unit` | Unit literal and type coloring. |
| `variable.quantity`, `property.quantity`, `parameter.quantity` | Quantity-bearing values and properties. |
| `parameter.declaration` | Function and args parameter declarations. |
| `variable.axis`, `property.axis` | Axis/workflow-step emphasis. |
| `type.timeseries`, `variable.timeseries`, `property.timeseries`, `function.timeseries` | TimeSeries type, value, and statistic helper emphasis. |
| `variable.uncertain`, `parameter.uncertain`, `function.uncertain`, `property.uncertain`, `keyword.uncertain` | Uncertainty values, dotted argument roots, functions, properties, and block introducers. |
| `keyword.defaultLibrary`, `function.defaultLibrary`, `namespace.defaultLibrary` | Built-in command-style keywords, helper functions, and modules. |
| `keyword.imported`, `namespace.imported`, `namespace.riskMedium` | Import statement keywords, user-imported module namespaces, and review-risk module dependencies. |
| `function.sideEffect`, `keyword.sideEffect`, `variable.sideEffect` | Side-effect operations and bindings. |
| `function.external`, `keyword.external`, `variable.external` | External boundaries and bindings. |
| `function.path` | Path construction and inspection helpers. |
| `function.temporal` | Supported calendar date construction helpers. |
| `keyword.validation`, `variable.validation`, `function.validation` | Validation and coverage operations. |
| `keyword.report`, `variable.report`, `property.report` | Report and plot operations. |
| `type.solver`, `class.solver`, `function.solver`, `keyword.solver`, `variable.solver`, `property.solver` | Solver types, declarations, fields, and equation operations. |
| `class.deprecated`, `property.deprecated` | Deprecated legacy declaration names and deprecated property roles. |
| `class.state`, `variable.state`, `property.state` | State-space type, system state, and state-member tokens. |
| `class.input`, `variable.input`, `parameter.input`, `property.input` | State-space input type, system inputs, input parameters, and input members. |
| `class.output`, `variable.output`, `property.output` | State-space output type, system outputs, output members, and output-like workflow values. |
| `type.model`, `variable.model`, `function.model`, `keyword.model`, `property.model`, `parameter.model` | Domain model types plus model and prediction artifacts, including `args.*` model roots. |
| `variable.db`, `keyword.db`, `function.db`, `method.db`, `property.db`, `parameter.db` | SQLite and DB-write boundaries, including `args.*` DB table targets. |
| `variable.cache`, `keyword.cache`, `function.cache`, `method.cache`, `property.cache` | Cache keys, cache helpers, cache option values, and records. |
| `keyword.workflowStep`, `function.workflowStep`, `variable.workflowStep`, `property.workflowStep` | Sampling, case, prediction, and workflow-step phrases. |
| `variable.riskHigh`, `variable.riskMedium`, `namespace.riskMedium`, `parameter.riskMedium`, `function.riskMedium` | Review-risk fallbacks that preserve the underlying symbol role. |
| `variable.planned`, `variable.internal`, `namespace.planned`, `namespace.internal` | Planned/internal symbol visibility. |

Base semantic selectors observed in LSP snapshots must keep fallback scopes even when a more specific modifier selector also exists. The package fallback map must also cover every token type and at least one selector for every modifier in the generated LSP legend; selectors for modifiers outside that legend are stale and must be removed. Keyword semantic selectors that represent command-style builtins, clause words, or option values must keep conventional keyword, operator, and constant fallbacks. In particular, `keyword.defaultLibrary` covers compiler-owned command words such as `sample`, `filter`, and `train`, `keyword.solver` covers solver command words plus clause words such as `over` and solver method literals such as `fixed_step`, `keyword.db` and `keyword.cache` keep DB/cache command words, clause words, and status literals on conventional keyword/operator/constant fallbacks, while `keyword.workflowStep` covers workflow words such as `read`, clause words such as `by`/`with`/`to`, validation-adjacent words such as `missing`, constants such as `asc`/`desc` and `true`/`false`, and builtin sampling methods such as `lhs`.

VS Code also applies a token-range dotted underline decoration for semantic
tokens carrying `planned` or `internal`. Current namespace coverage includes
source-visible stdlib module imports plus bundled stdlib namespace tokens. The
native IDE lexical fallback consumes `syntax_catalog.workflow_status_literals` without
a separate status-literal list, so `status =`, `status ==`, and `status !=`
literals keep workflow-step coloring before semantic tokens arrive. The native IDE
lexical fallback also consumes `syntax_catalog.units` plus generated
`syntax_catalog.legacy_unit_aliases` without a separate JavaScript unit list, so
numeric/unit coloring uses the compiler-owned unit catalog and editor-only
compatibility unit aliases before semantic tokens arrive.
Native IDE keyword coloring uses `syntax_catalog.keywords`,
`syntax_catalog.keyword_groups`, workflow builtin catalogs, and generated
`syntax_catalog.legacy_workflow_builtin_aliases` / `syntax_catalog.legacy_workflow_option_aliases`
for highlight-only compatibility spellings directly. Native
IDE constant coloring uses `syntax_catalog.constants` directly, so status,
policy, solver, and quality-status literals follow the LSP catalog. Native IDE
operator-word coloring uses `syntax_catalog.operator_words` directly, so words
such as `between`, `within`, and `matches` follow the LSP catalog.

The extension also contributes optional `EngLang Dark` and `EngLang Light`
color themes. Those themes define both TextMate fallback colors and direct
`semanticTokenColors` for every EngLang selector contributed in `package.json`,
including base selectors, standard modifiers, domain roles, and review-risk
roles. Domain role families such as units, quantities, TimeSeries, workflow
steps, validation, reporting, side effects, external boundaries, path and
temporal helpers, solver, model, DB, and cache keep a recognizable role hue while varying keyword,
function/method, variable, and property selectors so role-aware highlighting
does not collapse an entire workflow role into one flat color. Keep theme
selectors aligned with this fallback map; package validation
rejects theme-only semantic selectors without fallback mappings so users can
choose between their normal VS Code theme and a more colorful
EngLang-specific theme.

The fallback map currently references these TextMate scopes directly:

```text
comment.line.number-sign.englang
comment.line.documentation.englang
comment.line.double-slash.englang
constant.language.englang
constant.numeric.englang
constant.other.unit.englang
entity.name.function.englang
entity.name.function.workflow-step.englang
entity.name.function.solver.englang
invalid.deprecated.englang
keyword.control.deprecated.englang
keyword.control.external-boundary.englang
keyword.control.import.englang
keyword.control.model.englang
keyword.control.report.englang
keyword.control.side-effect.englang
keyword.control.solver.englang
keyword.control.validation.englang
keyword.control.workflow.englang
keyword.operator.englang
keyword.operator.word.englang
markup.warning.englang
meta.type.generic.englang
meta.type.array-suffix.englang
storage.type.declaration.englang
storage.type.function.englang
storage.type.test.englang
storage.type.block.englang
storage.type.interface-member.englang
storage.modifier.state.englang
storage.modifier.input.englang
storage.modifier.parameter.englang
storage.modifier.output.englang
storage.modifier.englang
storage.modifier.schema.englang
string.quoted.double.englang
support.function.builtin.englang
support.function.validation.englang
support.function.deprecated.englang
support.function.model.englang
support.function.uncertain.englang
support.function.timeseries.englang
support.function.external-boundary.englang
support.function.workflow-step.englang
support.function.solver.englang
support.function.path.englang
support.function.temporal.englang
support.namespace.module.englang
support.type.englang
variable.other.definition.englang
variable.other.model.englang
variable.other.state.englang
variable.other.input.englang
variable.other.output.englang
variable.other.parameter.englang
variable.other.property.englang
variable.parameter.type.englang
variable.parameter.argument.englang
variable.parameter.property.englang
```

When a new semantic modifier is added, update all of these together:

1. `crates/eng_lsp/src/lib.rs`
2. `tools/vscode-englang/package.json`
3. `tools/vscode-englang/generated/editor/englang-editor-metadata.json`
4. `tools/vscode-englang/generated/editor/englang-semantic-legend.json`
5. `tools/vscode-englang/generated/editor/englang-syntax.json`
6. This document
7. `.\dev.bat ide-check`

## Coverage Rules

Add or update grammar fixture expectations when a token is user-visible and a
theme should color it without semantic tokens. Add or update LSP snapshot
coverage when the role depends on compiler context, source metadata, review
risk, quantity/unit analysis, or workflow artifact semantics.

Historical editor batch notes belong in `docs/archive` or
`docs/current/usability_improvement_backlog.md`; keep this file focused on the
current token contract.
