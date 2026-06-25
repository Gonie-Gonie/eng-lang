# EngLang User Documentation

EngLang is a native engineering language for workflows where units, physical
quantities, schemas, time axes, plots, reports, and review artifacts are part
of the program rather than comments around the program.

Use this user documentation when you want to run EngLang, inspect an example,
or understand what the current portable package supports. Implementation
plans, solver internals, and historical release notes live elsewhere in the
repository.

## Start Here

1. [Getting started](getting_started.md)
2. [Install and doctor](tutorial/01_install_and_doctor.md)
3. [First unit-aware calculation](tutorial/02_first_unit_calculation.md)
4. [CSV, TimeSeries, plots, and report review](tutorial/04_schema_promote_csv.md)

## Tutorial Path

- [01 Install and doctor](tutorial/01_install_and_doctor.md)
- [02 First unit calculation](tutorial/02_first_unit_calculation.md)
- [03 Args and files](tutorial/03_args_and_files.md)
- [04 Schema and CSV promote](tutorial/04_schema_promote_csv.md)
- [05 TimeSeries statistics](tutorial/05_timeseries_statistics.md)
- [06 Plot, report, and review](tutorial/06_plot_report_review.md)
- [07 Validation and diagnostics](tutorial/07_validation_and_diagnostics.md)
- [08 Side effects and artifacts](tutorial/08_side_effects_and_artifacts.md)
- [09 Functions, imports, and const](tutorial/09_functions_imports_const.md)
- [10 Uncertainty basics](tutorial/10_uncertainty_basics.md)
- [11 Native IDE review](tutorial/11_native_ide_review.md)
- [12 Composite workflow](tutorial/12_composite_workflow.md)

## Task Guides

- [How-to guides](howto/README.md)
- [Concepts](concepts/README.md)
- [Composite workflow guide](../workflows/index.md)
- [Language grammar guide](../guide/language_grammar.md)
- [Standalone package reference](../reference/standalone_package.md)
- [Run command reference](../reference/cli_run.md)

## Documentation Tooling

Markdown in docs/user, docs/reference, docs/workflows, and docs/development is
the documentation source of truth. OODocs can publish curated Markdown into PDF
or other release artifacts, but it is not the EngLang reference extractor and
is not a runtime dependency.
