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
| VS Code semantic fallback scopes | `tools/vscode-englang/package.json` |
| Grammar smoke fixtures | `tools/vscode-englang/test/grammar-fixtures/*.eng` |
| Grammar smoke expectations | `tools/vscode-englang/test/expected/grammar_tokens.json` |

Edit the source grammar, not the generated grammar. After grammar changes run:

```bat
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
| `comment.line.*.englang` | Line comments and documentation comments. |
| `keyword.control.*.englang` | Workflow, report, validation, solver, deprecated, side-effect, and external-boundary words. |
| `keyword.operator*.englang` | Word and symbolic operators. |
| `punctuation.section.*.englang` | Block, bracket, and parenthesis delimiters. |
| `punctuation.separator.*.englang` | Separators such as commas and colons. |
| `punctuation.accessor.*.englang` | Accessor punctuation such as dots in paths or members. |
| `storage.type.*.englang` | Block openers such as `args {`, `where {`, and declaration-level markers. |
| `storage.modifier.*.englang` | Modifiers such as schema indexes or system member roles. |
| `storage.modifier.englang` | State and input fallback coloring from semantic token mappings. |
| `entity.name.type.declaration.englang` | Full type-like declaration phrases such as `schema SensorData`. |
| `entity.name.type.englang` | Captured declaration names after `schema`, `system`, `domain`, `component`, and `class`. |
| `meta.declaration.function.englang` | Full `fn` and `method` declaration phrases. |
| `entity.name.function.englang` | Captured `fn` and `method` names. |
| `entity.name.function.call.englang` | User-defined function call names. |
| `entity.name.function.workflow-step.englang` | Workflow step names and compiler-backed workflow-step semantic fallbacks. |
| `meta.declaration.constant.englang` | Full `const` declaration phrases. |
| `variable.other.constant.englang` | Captured `const` names. |
| `meta.declaration.test.englang` | Full `test "..."` declaration phrases. |
| `entity.name.section.englang` | Captured quoted test names. |
| `support.type.englang` | Quantity/type names. |
| `meta.type.generic.englang` | Generic type expressions such as `Table[T]`, `Optional[DirectoryPath]`, and `TimeSeries[Time]`. |
| `variable.parameter.type.englang` | Generic type arguments inside bracketed type expressions. |
| `support.function.builtin.englang` | Built-in functions and helpers. |
| `support.namespace.module.englang` | Module namespaces such as `eng.table`. |
| `constant.numeric*.englang` | Numeric literals and format precision fragments. |
| `constant.other.unit*.englang` | Unit literals. |
| `constant.language.englang` | Language constants and uncertainty/fallback words. |
| `variable.parameter.property.englang` | `args.*`, `with` options, and parameter-like property paths. |
| `variable.parameter.function.englang` | Function and method parameter declarations plus named call arguments such as `std=` or `target=`. |
| `variable.language.self.englang` | `self` references inside class methods. |
| `variable.other.local.englang` | Runtime binding references inside workflow phrases. |
| `meta.declaration.parameter.englang` | Function and method parameter declaration fragments. |
| `meta.declaration.typed-binding.englang` | Typed value declarations written with `name: Type = ...`. |
| `meta.declaration.field.englang` | Field declarations written with `name:`. |
| `variable.other.property.englang` | Schema/class fields, component members, property paths, and object fields. |
| `variable.other.definition.englang` | Runtime binding names written with `name = ...`. |
| `meta.workflow.*.englang` | Phrase scopes for multi-token workflow operations. |
| `meta.report.*.englang` | Report phrase scopes. |
| `meta.quantity.literal.englang` | Unit-bearing numeric expressions. |
| `meta.interpolation.englang` | String interpolation bodies. |
| `invalid.deprecated.englang` | High-risk fallback mapping. |
| `markup.warning.englang` | Medium-risk fallback mapping. |

Prefer adding a phrase-level `meta.workflow.*.englang` scope when a native
workflow operation is more readable as a single action than as unrelated
keywords. Examples include `sample lhs`, `predict model using`, `read json`,
`open sqlite`, and `write ... to db.table(...)`.

Current workflow phrase scopes:

| Scope | Phrase |
| --- | --- |
| `meta.workflow.apply-call.englang` | `apply(<step>, over=<table>)` |
| `meta.workflow.apply-step.englang` | `apply <step> over <table>` |
| `meta.workflow.align-series.englang` | `align <series> with <series>` |
| `meta.workflow.check-coverage.englang` | `check coverage <series>` |
| `meta.workflow.collect-results.englang` | `collect results <table>` |
| `meta.workflow.db-write.englang` | `write <table> to <db>.table("<name>")` |
| `meta.workflow.derive-column.englang` | `derive <table> column ...` |
| `meta.workflow.distribution-call.englang` | `uniform(...)`, `normal(...)`, `measured(...)`, and `interval(...)` value distributions. |
| `meta.workflow.download-to.englang` | `download ... to ...` |
| `meta.workflow.export-summary-csv.englang` | `export summary to csv <target>` |
| `meta.workflow.filter-table.englang` | `filter <table>` |
| `meta.workflow.http-request.englang` | `http get <target>`, `http post <target>`, and other HTTP request phrases. |
| `meta.workflow.integrate-call.englang` | `integrate(<series>, over=<axis>)` |
| `meta.workflow.integrate-series.englang` | `integrate <series> over <axis>` |
| `meta.workflow.join-table.englang` | `join <left> with <right>` |
| `meta.workflow.log-message.englang` | `log <level> "..."` structured runtime message lines. |
| `meta.workflow.materialize-cases.englang` | `materialize cases <table>` |
| `meta.workflow.model-summary-call.englang` | `evaluate(<model>)`, `model_card(<model>)`, and related model summary calls. |
| `meta.workflow.open-sqlite.englang` | `open sqlite <source>` |
| `meta.workflow.option-map.englang` | `query = { ... }`, `headers = { ... }`, and `values = { ... }` option maps. |
| `meta.workflow.with-block.englang` | `with { ... }` option blocks scoped separately from top-level bindings. |
| `meta.workflow.predict-model.englang` | `predict <model> using <table>` |
| `meta.workflow.print-message.englang` | `print "..."` runtime message lines. |
| `meta.workflow.plot-distribution.englang` | `plot distribution(<distribution>)` |
| `meta.workflow.plot-series.englang` | `plot <series> over <axis>` |
| `meta.workflow.promote-csv.englang` | `promote csv <source> as <schema>` |
| `meta.workflow.promote-json.englang` | `promote json <source> as <schema>` |
| `meta.workflow.promote-json-records.englang` | `promote json records <source> as <schema>` |
| `meta.workflow.promote-toml.englang` | `promote toml <source> as <schema>` |
| `meta.workflow.read-structured.englang` | `read json <source>`, `read toml <source>`, `read text <source>`, and `read csv <source>`. |
| `meta.workflow.regression-table.englang` | `regression_table(<table>, target=..., features=..., ...)` |
| `meta.workflow.require-one.englang` | `require_one <table>` |
| `meta.workflow.resample-series.englang` | `resample <series> to <series>` |
| `meta.workflow.render-template.englang` | `render template <source>` |
| `meta.workflow.return-statement.englang` | `return <value>` function return lines. |
| `meta.workflow.run-command.englang` | `run command ...` |
| `meta.workflow.sample-method.englang` | `sample lhs`, `sample grid`, `sample random`, and related sample methods. |
| `meta.workflow.select-columns.englang` | `select <table> columns <columns>` |
| `meta.workflow.show-report.englang` | `show <value>` and optional report display suffixes. |
| `meta.workflow.sort-table.englang` | `sort <table> by <column> [asc|desc]` |
| `meta.workflow.stat-axis-call.englang` | `mean(<series>, axis=<axis>)`, `max(<series>, axis=<axis>)`, and related axis statistic calls. |
| `meta.workflow.stat-series.englang` | `mean <series> over <axis>`, `max <series> over <axis>`, and related command-style statistic phrases. |
| `meta.workflow.status-condition.englang` | `status == passed` and related `on { ... }` status checks. |
| `meta.workflow.summary-field.englang` | `<value> as <unit> with "<format>"` summary CSV fields. |
| `meta.workflow.summarize-series.englang` | `summarize <series> by [...]` |
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

The current EngLang-specific modifiers are:

```text
declaration, definition, readonly, static, local, imported, defaultLibrary,
deprecated, unit, quantity, axis, timeseries, uncertain, sideEffect, external,
validation, report, solver, planned, internal, riskHigh, riskMedium, state,
input, model, db, cache, workflowStep
```

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
| `model` | Model training, evaluation, prediction, and model-card artifacts. |
| `db` | SQLite connections, table writes, and DB boundary records. |
| `cache` | Cache declarations, keys, and cache-backed workflow records. |
| `workflowStep` | Case orchestration, sampling, prediction, and workflow-step symbols. |

Core semantic role expectations:

| Source role | Semantic token |
| --- | --- |
| `use`/`import` namespace | `namespace` with `declaration` and `imported`. |
| `const` name | `variable` with `declaration` and `readonly`. |
| Schema name | `class` with `declaration`. |
| Schema/class/component field | `property` with `declaration`. |
| `args` field | `parameter` with `declaration`. |
| Function parameter | `parameter` with `declaration`. |
| Function-local binding | `variable` with `local`. |
| Deprecated `script`/`struct` keyword | `keyword` with `deprecated`. |
| Bundled stdlib domain namespace | `namespace` with `defaultLibrary` and `internal`. |

`planned` is currently reserved for future source-visible module or workflow
symbols. Add LSP regression coverage and a VS Code fallback mapping in the same
change that starts emitting it from source.

## VS Code Fallback Mapping

VS Code maps semantic tokens to TextMate fallback scopes in
`tools/vscode-englang/package.json` under `semanticTokenScopes`. Keep that map
in sync with the generated legend. Important pairings:

| Semantic selector | Fallback scope intent |
| --- | --- |
| `type` | Type names, generic type expressions, and bracketed type arguments. |
| `type.unit`, `property.unit` | Unit literal and type coloring. |
| `variable.quantity`, `property.quantity`, `parameter.quantity` | Quantity-bearing values and properties. |
| `parameter.declaration` | Function and args parameter declarations. |
| `variable.axis`, `property.axis` | Axis/workflow-step emphasis. |
| `variable.timeseries`, `property.timeseries` | TimeSeries value emphasis. |
| `function.defaultLibrary`, `namespace.defaultLibrary` | Built-in functions and modules. |
| `namespace.imported` | User-imported module namespaces. |
| `function.sideEffect`, `keyword.sideEffect`, `variable.sideEffect` | Side-effect operations and bindings. |
| `function.external`, `keyword.external`, `variable.external` | External boundaries and bindings. |
| `keyword.validation`, `variable.validation`, `function.validation` | Validation and coverage operations. |
| `keyword.report`, `variable.report`, `property.report` | Report and plot operations. |
| `function.solver`, `keyword.solver`, `variable.solver` | Solver and equation operations. |
| `variable.state`, `property.state` | System state tokens. |
| `variable.input`, `parameter.input` | System inputs and input parameters. |
| `variable.model`, `function.model`, `property.model` | Model and prediction artifacts. |
| `variable.db`, `keyword.db` | SQLite and DB-write boundaries. |
| `variable.cache`, `property.cache` | Cache keys and records. |
| `keyword.workflowStep`, `function.workflowStep`, `variable.workflowStep`, `property.workflowStep` | Sampling, case, prediction, and workflow-step phrases. |
| `variable.riskHigh`, `variable.riskMedium` | Review-risk fallbacks. |
| `variable.planned`, `variable.internal`, `namespace.internal` | Planned/internal symbol visibility. |

VS Code also applies a token-range dotted underline decoration for semantic
tokens carrying `planned` or `internal`. `planned` remains reserved until a
source-visible planned symbol path exists; current coverage is primarily
bundled stdlib namespace tokens such as `namespace.internal`.

The fallback map currently references these TextMate scopes directly:

```text
comment.line.number-sign.englang
constant.language.englang
constant.other.unit.englang
entity.name.function.englang
entity.name.function.workflow-step.englang
invalid.deprecated.englang
keyword.control.deprecated.englang
keyword.control.external-boundary.englang
keyword.control.report.englang
keyword.control.side-effect.englang
keyword.control.solver.englang
keyword.control.validation.englang
keyword.control.workflow.englang
markup.warning.englang
meta.type.generic.englang
storage.modifier.englang
support.function.builtin.englang
support.namespace.module.englang
support.type.englang
variable.other.definition.englang
variable.other.property.englang
variable.parameter.type.englang
variable.parameter.property.englang
```

When a new semantic modifier is added, update all of these together:

1. `crates/eng_lsp/src/lib.rs`
2. `tools/vscode-englang/package.json`
3. `tools/vscode-englang/generated/editor/englang-editor-metadata.json`
4. `tools/vscode-englang/generated/editor/englang-semantic-legend.json`
5. This document
6. `.\dev.bat ide-check`

## Coverage Rules

Add or update grammar fixture expectations when a token is user-visible and a
theme should color it without semantic tokens. Add or update LSP snapshot
coverage when the role depends on compiler context, source metadata, review
risk, quantity/unit analysis, or workflow artifact semantics.
