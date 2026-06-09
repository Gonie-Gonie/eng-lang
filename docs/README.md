# EngLang Documentation

This index maps the current repository status to the concrete documentation
needed for implementation. Start with the current-status layer before opening
long-form planning documents.

## Start Here

1. [Current project status](current/status.md)
2. [Integrated language philosophy](current/philosophy.md)
3. [Version plan](current/version_plan.md)
4. [Feature maturity matrix](current/feature_maturity_matrix.md)
5. [Development tracks](current/tracks.md)
6. [Standalone package reference](reference/standalone_package.md)
7. [LLM load map](llm/load_map.yml)
8. [Getting started](development/00_getting_started.md)
9. [Repository layout](development/01_repo_layout.md)
10. [Daily workflow](development/02_daily_workflow.md)
11. [Reproducible environment policy](development/03_environment_reproducibility.md)
12. [Version roadmap workflow](development/04_version_roadmap_workflow.md)
13. [Curated user documentation source](user/README.md)

## Architecture

- [System overview](architecture/00_system_overview.md)
- [Runtime artifacts](architecture/01_runtime_artifacts.md)
- [Compiler frontend](architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](architecture/03_expected_types_and_quantities.md)
- [Data boundary and CSV promote](architecture/04_data_boundary.md)
- [Bytecode VM and result v1](runtime/bytecode.md)
- [Language grammar guide](guide/language_grammar.md)
- [Integrated language philosophy](current/philosophy.md)
- [Side effect and general programming policy](reference/side_effect_policy.md)
- [TimeSeries statistics guide](guide/timeseries_statistics.md)
- [Plotting guide](guide/plotting.md)
- [Uncertainty track guide](guide/uncertainty.md)
- [Data-driven modeling track guide](guide/data_driven_modeling.md)
- [Domain and component track guide](guide/domain_component.md)
- [Native tester IDE](guide/native_ide.md)
- [Report and review artifacts](guide/report_review.md)
- [Simple system tutorial](tutorials/05_simple_system.md)
- [Integrated HVAC user test](tutorials/06_integrated_hvac.md)
- [Curated user documentation source](user/README.md)

## Reference

- [CLI specification](specs/cli.md)
- [Run command reference](reference/cli_run.md)
- [Standalone package reference](reference/standalone_package.md)
- [Side effect and general programming policy](reference/side_effect_policy.md)
- [LSP snapshot reference](reference/lsp_snapshot.md)
- [Kernel plan reference](reference/kernel_plan.md)
- [JIT benchmark harness reference](reference/jit_benchmark.md)
- [v8/v9 language policy](specs/language-v8.md)
- [Fast assignment guide](language/fast_assignment.md)
- [Dimensionless policy guide](language/dimensionless.md)
- [Language grammar guide](guide/language_grammar.md)
- [Unit-aware print and CSV summary export](language/string_formatting.md)

## Planning And Release

- [Roadmap](roadmap.md)
- [Current planning pointer](master-plan/current.md)
- [Release acceptance checklist](release/acceptance-checklist.md)
- [Release workflow](release/release-workflow.md)
- [v0.1-preview release notes](release/v0.1-preview.md)
- [v0.2-preview release notes](release/v0.2-preview.md)
- [v0.3-preview release notes](release/v0.3-preview.md)
- [v0.4-preview release notes](release/v0.4-preview.md)
- [v0.5-preview release notes](release/v0.5-preview.md)
- [v0.6-preview release notes](release/v0.6-preview.md)
- [v0.7-preview release notes](release/v0.7-preview.md)
- [v0.8-preview release notes](release/v0.8-preview.md)
- [v0.9-preview release notes](release/v0.9-preview.md)
- Long-form v9 plan is linked from the planning pointer only as historical
  technical intent; do not use its old milestone labels as release names.

## Documentation Rules

- Public behavior changes must update README, current status, maturity matrix,
  examples, and release notes.
- Runtime artifact changes must update [Runtime artifacts](architecture/01_runtime_artifacts.md).
- Bytecode/result changes must update [Bytecode VM and result v1](runtime/bytecode.md).
- Core path changes must not add Python or interpreter dependencies.
- Portable release packages should ship curated user PDFs, not the full
  developer markdown tree.
- Public release versions and future development tracks must remain separate.
