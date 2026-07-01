# EngLang Editor Token Scopes

This note is the maintainer-facing contract for EngLang highlighting. It
documents the current TextMate fallback scopes, semantic token modifiers, and
the files that keep VS Code and the native IDE aligned.

## Source Of Truth

| Surface | Source |
| --- | --- |
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
| `storage.type.*.englang` | Block openers such as `args {` and declaration-level markers. |
| `storage.modifier.*.englang` | Modifiers such as schema indexes or system member roles. |
| `entity.name.type.declaration.englang` | Full type-like declaration phrases such as `schema SensorData`. |
| `entity.name.type.englang` | Captured declaration names after `schema`, `system`, `domain`, `component`, and `class`. |
| `meta.declaration.function.englang` | Full `fn` and `method` declaration phrases. |
| `entity.name.function.englang` | Captured `fn` and `method` names. |
| `meta.declaration.constant.englang` | Full `const` declaration phrases. |
| `variable.other.constant.englang` | Captured `const` names. |
| `meta.declaration.test.englang` | Full `test "..."` declaration phrases. |
| `entity.name.section.englang` | Captured quoted test names. |
| `support.type.englang` | Quantity/type names. |
| `support.function.builtin.englang` | Built-in functions and helpers. |
| `support.namespace.module.englang` | Module namespaces such as `eng.table`. |
| `constant.numeric*.englang` | Numeric literals and format precision fragments. |
| `constant.other.unit*.englang` | Unit literals. |
| `constant.language.englang` | Language constants and uncertainty/fallback words. |
| `variable.parameter.property.englang` | `args.*`, `with` options, and parameter-like property paths. |
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
| `sideEffect` | Filesystem or other declared side effects. |
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

## VS Code Fallback Mapping

VS Code maps semantic tokens to TextMate fallback scopes in
`tools/vscode-englang/package.json` under `semanticTokenScopes`. Keep that map
in sync with the generated legend. Important pairings:

| Semantic selector | Fallback scope intent |
| --- | --- |
| `type.unit`, `property.unit` | Unit literal and type coloring. |
| `variable.quantity`, `property.quantity`, `parameter.quantity` | Quantity-bearing values and properties. |
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
| `keyword.workflowStep`, `function.workflowStep`, `property.workflowStep` | Sampling, case, prediction, and workflow-step phrases. |
| `variable.riskHigh`, `variable.riskMedium` | Review-risk fallbacks. |
| `variable.planned`, `variable.internal`, `namespace.internal` | Planned/internal symbol visibility. |

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
