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
| Native IDE lexical fallback catalog | `eng-lsp --editor-metadata` via `ide_bootstrap.syntaxCatalog` |
| VS Code semantic fallback scopes | `tools/vscode-englang/package.json` |
| Optional VS Code color themes | `tools/vscode-englang/themes/englang-dark-color-theme.json` and `tools/vscode-englang/themes/englang-light-color-theme.json` |
| Grammar smoke fixtures | `tools/vscode-englang/test/grammar-fixtures/*.eng` |
| Grammar smoke expectations | `tools/vscode-englang/test/expected/grammar_tokens.json` |

Edit the source grammar, not the generated grammar. The source grammar may use
`{{...}}` placeholders for compiler-owned keyword, workflow helper, option,
type, and unit lists; `build-grammar.ps1` expands them from generated editor
metadata, including `syntax_catalog.workflow_status_literals` for `status ==`, `status !=`, and `status =` values. After grammar changes run:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat vscode-build-grammar
.\dev.bat vscode-grammar-test
```

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
| `storage.modifier.operator.englang` | `operator` member keyword fallback coloring aligned with solver semantic colors. |
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
| `support.function.builtin.englang` | Built-in functions and helpers. |
| `support.function.model.englang` | Model-training and model-summary helper calls such as `regression(...)`, `train_test_split(...)`, `evaluate(...)`, and `model_card(...)`. |
| `support.function.uncertain.englang` | Uncertainty helper calls such as `measured(...)`, `uniform(...)`, `propagate(...)`, and `probability(...)`. |
| `support.function.timeseries.englang` | TimeSeries/statistic helper calls such as `integrate(...)`, `mean(...)`, `time_weighted_mean`, and `p90`. |
| `support.function.external-boundary.englang` | External boundary constructors/checks such as `file(...)`, `url(...)`, `env(...)`, `secret env(...)`, and `exists(...)`. |
| `support.function.workflow-step.englang` | Workflow-step helper calls such as `apply(...)` and step values such as `run_case`. |
| `support.function.solver.englang` | Solver helper calls such as `der(...)`. |
| `support.function.path.englang` | Path helper calls such as `join(...)`. |
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

Command-style workflow and review verbs such as `sample`, `filter`, `derive`, `require_one`, `integrate`, and `mean` use `keyword.control.*.englang`; model workflow phrases such as `predict ... using ...` and `train regression ...` use `keyword.control.model.englang`; model helper calls such as `regression(...)`, `train_test_split(...)`, `evaluate(...)`, and `model_card(...)` use `support.function.model.englang`; uncertainty helper calls such as `measured(...)`, `uniform(...)`, `propagate(...)`, and `probability(...)` use `support.function.uncertain.englang`; TimeSeries/statistic helper calls such as `integrate(...)`, `mean(...)`, `time_weighted_mean`, and `p90` use `support.function.timeseries.englang`; external boundary constructors/checks such as `file(...)`, `url(...)`, `env(...)`, `secret env(...)`, and `exists(...)` use `support.function.external-boundary.englang`; workflow-step helpers such as `apply(...)` and `run_case` use `support.function.workflow-step.englang`; solver helpers such as `der(...)` use `support.function.solver.englang`; path helpers such as `join(...)` use `support.function.path.englang`; TimeSeries quality verbs such as `fill`, `align`, and `resample` use validation-colored fallback scopes to match their phrase scopes. Other generic call-style helpers stay under `support.function.builtin.englang`.

`#members` must appear before generic `args.*` and dotted-path fallbacks inside expression contexts so TextMate first-paint tokenization can split roots, dots, and member segments before broad property regexes match the whole path. Grammar smoke also requires begin/end workflow phrase scopes to include `#members`, so operand-oriented phrases cannot regress to uncolored dotted paths.

Prefer adding a phrase-level `meta.workflow.*.englang` scope when a native
workflow operation is more readable as a single action than as unrelated
keywords. Examples include `sample lhs`, `predict model using`, `read json`,
`open sqlite`, and `write ... to db.table(...)`.

For `promote csv/json/toml` and `promote json records`, TextMate first-paint
scopes keep the phrase workflow-colored and include member-aware source-path
fallbacks, while LSP semantic tokens add `workflowStep` plus `external` to
source operands such as `file(...)`, `args.input`, `payload`, and
`payload.records`.

I/O and external-boundary phrases such as `read json`, `write text`,
`write json`, `write standard_text`, `export summary to csv`, `download`,
`http ...`, `render template`, and file operations also keep `#members`
before broad `args.*` and dotted-path fallbacks so source and target operands
split consistently before semantic highlighting arrives.

DB/table phrases such as `open sqlite`, `read sqlite <db>.table(...)`,
`write <table> to <db>.table(...)`, and `select <table> columns ...` use the
same member-aware fallback ordering so nested DB/table receivers and selected
source tables split before broad property scopes.

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
`evaluate(...)`, `model_card(...)`, and `leakage_lint(...)` also include
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
| `meta.workflow.model-train-call.englang` | `train_test_split(...)`, legacy-compatible `train_regression(...)`, `regression(...)`, `mlp(...)`, and `ann(...)` model-training calls. |
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
| `meta.workflow.regression-table.englang` | Legacy-compatible `regression_table(<table>, target=..., features=..., ...)` table-model training calls. |
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
| `meta.workflow.rmse-comparison.englang` | `rmse <measured-series> vs <simulated-series>` command-style comparison phrases; includes member-aware first-paint fallbacks for dotted measured/simulated operands. |
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
declaration, definition, readonly, static, local, imported, defaultLibrary,
deprecated, documentation, unit, quantity, axis, timeseries, uncertain, sideEffect, external,
validation, report, solver, planned, internal, riskHigh, riskMedium, state,
input, output, model, db, cache, workflowStep
```

Modifier meanings:

| Modifier | Meaning |
| --- | --- |
| `static` | Static-like or class-level roles when the compiler emits the standard VS Code modifier. |
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

Core semantic role expectations:

| Source role | Semantic token |
| --- | --- |
| `use`/`import` keyword | `keyword` with `declaration` and `imported`. |
| `use`/`import` namespace | `namespace` with `declaration` and `imported`. |
| `const` name | `variable` with `declaration` and `readonly`. |
| Schema name | `class` with `declaration`. |
| Schema/class/component field | `property` with `declaration`. |
| `args` field | `parameter` with `declaration`. |
| Function parameter | `parameter` with `declaration`. |
| System state declaration | `variable` with `declaration` and `state`. |
| System input declaration | `variable` with `declaration` and `input`. |
| System parameter declaration | `parameter` with `declaration` and `readonly`. |
| System output declaration | `variable` with `declaration` and `output`. |
| Function-local binding | `variable` with `local`. |
| Deprecated `script`/`struct` keyword | `keyword` with `deprecated`. |
| Deprecated `script`/`struct` declaration name | `class` with `declaration` and `deprecated`. |
| Bundled stdlib domain namespace | `namespace` with `defaultLibrary` and `internal`. |
| Supported/native `eng.*` module import | `namespace` with `declaration`, `imported`, and `defaultLibrary`. |
| Planned `eng.*` module import | `namespace` with `declaration`, `imported`, `defaultLibrary`, and `planned`. |
| Internal `eng.*` module import | `namespace` with `declaration`, `imported`, `defaultLibrary`, and `internal`. |
| String interpolation expression | `variable`/`parameter` plus `property` path segments inside `{...}` fields; format precision digits use `number`, and format units use `type` with `unit`. |

`planned` is currently emitted for source-visible planned stdlib module imports
such as `use eng.stats`; internal stdlib imports such as `use eng.system` carry
`internal`.

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
| `class`, `interface`, `class.declaration`, `interface.declaration` | Declared schema, system, component, domain, and interface-like names. |
| `class.defaultLibrary`, `interface.defaultLibrary` | Bundled type/domain names surfaced by the compiler. |
| `comment` | Ordinary (`#`, `//`) and documentation (`///`) line comments. |
| `comment.documentation` | Documentation comments (`///`) when semantic highlighting is available. |
| `function`, `function.declaration`, `function.definition`, `method`, `method.declaration` | User-defined function and method calls plus declaration names. |
| `function.report` | Report helper functions when emitted as semantic function tokens. |
| `keyword`, `keyword.declaration`, `keyword.local` | General workflow keywords, declaration keywords, schema-index modifier fallbacks, local keyword-like roles, and block-opener semantic fallbacks. |
| `modifier`, `modifier.static` | Modifier-like role tokens, schema indexes, and static modifier fallbacks. |
| `namespace`, `namespace.declaration` | Imported or declared module namespaces. |
| `number` | Numeric literals. |
| `parameter`, `parameter.readonly` | Function parameters, args-like parameters, and read-only parameter roles. |
| `property`, `property.declaration`, `property.readonly` | Property paths, declared schema/class/system fields, and read-only property roles. |
| `variable`, `variable.local`, `variable.declaration`, `variable.defaultLibrary`, `variable.readonly`, `variable.deprecated`, `variable.static` | Plain variables, local references, declared bindings, bundled value symbols, read-only constants, deprecated variables, and static-like values. |
| `type.unit`, `property.unit` | Unit literal and type coloring. |
| `variable.quantity`, `property.quantity`, `parameter.quantity` | Quantity-bearing values and properties. |
| `parameter.declaration` | Function and args parameter declarations. |
| `variable.axis`, `property.axis` | Axis/workflow-step emphasis. |
| `variable.timeseries`, `property.timeseries`, `function.timeseries` | TimeSeries value and statistic helper emphasis. |
| `variable.uncertain`, `function.uncertain`, `property.uncertain`, `keyword.uncertain` | Uncertainty values, functions, properties, and block introducers. |
| `keyword.defaultLibrary`, `function.defaultLibrary`, `namespace.defaultLibrary` | Built-in command-style keywords, helper functions, and modules. |
| `keyword.imported`, `namespace.imported` | Import statement keywords and user-imported module namespaces. |
| `function.sideEffect`, `keyword.sideEffect`, `variable.sideEffect` | Side-effect operations and bindings. |
| `function.external`, `keyword.external`, `variable.external` | External boundaries and bindings. |
| `keyword.validation`, `variable.validation`, `function.validation` | Validation and coverage operations. |
| `keyword.report`, `variable.report`, `property.report` | Report and plot operations. |
| `function.solver`, `keyword.solver`, `variable.solver` | Solver and equation operations. |
| `class.deprecated`, `property.deprecated` | Deprecated legacy declaration names and deprecated property roles. |
| `variable.state`, `property.state` | System state tokens. |
| `variable.input`, `parameter.input` | System inputs and input parameters. |
| `variable.output` | System outputs and output-like workflow values. |
| `variable.model`, `function.model`, `keyword.model`, `property.model` | Model and prediction artifacts. |
| `variable.db`, `keyword.db`, `function.db`, `method.db`, `property.db`, `parameter.db` | SQLite and DB-write boundaries, including `args.*` DB table targets. |
| `variable.cache`, `keyword.cache`, `function.cache`, `method.cache`, `property.cache` | Cache keys, cache helpers, cache option values, and records. |
| `keyword.workflowStep`, `function.workflowStep`, `variable.workflowStep`, `property.workflowStep` | Sampling, case, prediction, and workflow-step phrases. |
| `variable.riskHigh`, `variable.riskMedium` | Review-risk fallbacks. |
| `variable.planned`, `variable.internal`, `namespace.planned`, `namespace.internal` | Planned/internal symbol visibility. |

Base semantic selectors observed in LSP snapshots must keep fallback scopes even when a more specific modifier selector also exists. The package fallback map must also cover every token type and at least one selector for every modifier in the generated LSP legend, including standard modifiers such as `static` before they become common in emitted tokens. Keyword semantic selectors that represent command-style builtins, clause words, or option values must keep conventional keyword, operator, and constant fallbacks. In particular, `keyword.defaultLibrary` covers compiler-owned command words such as `sample`, `filter`, and `train`, `keyword.solver` covers solver command words plus clause words such as `over` and solver method literals such as `fixed_step`, `keyword.db` and `keyword.cache` keep DB/cache command words, clause words, and status literals on conventional keyword/operator/constant fallbacks, while `keyword.workflowStep` covers workflow words such as `read`, clause words such as `by`/`with`/`to`, validation-adjacent words such as `missing`, constants such as `asc`/`desc` and `true`/`false`, and builtin sampling methods such as `lhs`.

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
steps, validation, reporting, side effects, external boundaries, solver, model,
DB, and cache keep a recognizable role hue while varying keyword,
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
storage.modifier.operator.englang
storage.modifier.englang
storage.modifier.schema.englang
string.quoted.double.englang
support.function.builtin.englang
support.function.model.englang
support.function.uncertain.englang
support.function.timeseries.englang
support.function.external-boundary.englang
support.function.workflow-step.englang
support.function.solver.englang
support.function.path.englang
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
