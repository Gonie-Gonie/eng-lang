# Daily Development Workflow

This document describes the normal development loop for EngLang.

## 1. Check Current State

```bat
git status --short
.\dev.bat doctor
.\dev.bat test
```

Fix setup or environment problems before starting feature work.

## 2. Pick the Version Target

Development follows the v9 roadmap. Start with the target version, then read the
required outputs and release gate for that version.

Useful questions:

```text
1. Is this a language change?
2. Is this a compiler/frontend change?
3. Is this a runtime/artifact change?
4. Is this a tooling or packaging change?
5. Is this product documentation or an example?
```

Common locations:

```text
diagnostics and semantic checks    crates/eng_compiler
report sections                    crates/eng_report
eng command behavior               crates/eng_cli
run/build artifact layout          crates/eng_runtime plus docs
setup and packaging workflow       scripts/dev.ps1, dev.bat, development docs
```

## 3. Implement in Reviewable Units

Preferred order:

```text
1. add or select an example/error case
2. update compiler/runtime/report behavior
3. update docs and release notes
4. run .\dev.bat fmt
5. run .\dev.bat test
6. run .\dev.bat clippy when the change touches Rust behavior
```

Commit after each independently useful unit that passes the relevant checks.

## 4. Keep Public Examples Current

Current official examples:

```text
examples/01_units/main.eng              unit/quantity basics
examples/official/01_csv_plot/main.eng  typed CSV, statistics, PlotSpec, report
examples/official/02_simple_system/main.eng
                                           minimal system/equation report
```

Current error examples:

```text
examples/05_error_messages/unit_mismatch.eng
examples/05_error_messages/ambiguous_power.eng
examples/05_error_messages/heat_rate_sum.eng
examples/05_error_messages/missing_csv_column.eng
examples/05_error_messages/eq_boolean.eng
examples/05_error_messages/equation_unit_mismatch.eng
examples/05_error_messages/missing_entry.eng
```

Public behavior should have an example, a smoke test, or both.

## 5. Update the Right Docs

```text
CLI output or options          docs/specs/cli.md
language syntax or policy      docs/specs/language-v8.md
artifact layout                docs/architecture/01_runtime_artifacts.md
setup or packaging             docs/development/00_getting_started.md
repo structure                 docs/development/01_repo_layout.md
daily workflow                 docs/development/02_daily_workflow.md
environment reproducibility    docs/development/03_environment_reproducibility.md
milestone scope                docs/roadmap.md
release gates                  docs/release/acceptance-checklist.md
```

Runtime artifact changes should also update the release notes for the active
milestone.

## 6. Verify Before Commit

For a normal development slice:

```bat
.\dev.bat ci
```

For documentation/spec snippets:

```bat
.\dev.bat docs-check
```

For artifact schema or golden baseline changes:

```bat
.\dev.bat artifacts-check
```

For packaging/release work:

```bat
.\dev.bat release-check
```

If the change generates visual/report artifacts, inspect `build/result/report.html`
and the generated PlotSpec/report files before release.
