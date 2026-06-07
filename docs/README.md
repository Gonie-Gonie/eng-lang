# EngLang Documentation

This index maps the current repository status to the concrete documentation
needed for implementation. Start with the current-status layer before opening
long-form planning documents.

## Start Here

1. [Current project status](current/status.md)
2. [Feature maturity matrix](current/feature_maturity_matrix.md)
3. [v1.0.3 hardening register](current/v1_0_3_hardening.md)
4. [v1.2 data-driven modeling gate](current/v1_2_data_driven_modeling_gate.md)
5. [v1.3 LSP gate](current/v1_3_lsp_gate.md)
6. [v1.4 JIT gate](current/v1_4_jit_gate.md)
7. [v1.5 standalone/AOT gate](current/v1_5_standalone_gate.md)
8. [v2.0 domain/component gate](current/v2_0_domain_component_gate.md)
9. [Standalone package reference](reference/standalone_package.md)
10. [LLM load map](llm/load_map.yml)
11. [Getting started](development/00_getting_started.md)
12. [Repository layout](development/01_repo_layout.md)
13. [Daily workflow](development/02_daily_workflow.md)
14. [Reproducible environment policy](development/03_environment_reproducibility.md)
15. [Version roadmap workflow](development/04_version_roadmap_workflow.md)
16. [Curated user documentation source](user/README.md)

## Architecture

- [System overview](architecture/00_system_overview.md)
- [Runtime artifacts](architecture/01_runtime_artifacts.md)
- [Compiler frontend](architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](architecture/03_expected_types_and_quantities.md)
- [Data boundary and CSV promote](architecture/04_data_boundary.md)
- [Bytecode VM and result v1](runtime/bytecode.md)
- [TimeSeries statistics guide](guide/timeseries_statistics.md)
- [Plotting guide](guide/plotting.md)
- [Uncertainty core guide](guide/uncertainty.md)
- [Data-driven modeling guide](guide/data_driven_modeling.md)
- [Domain and component guide](guide/domain_component.md)
- [Native tester IDE](guide/native_ide.md)
- [Report and review artifacts](guide/report_review.md)
- [Simple system tutorial](tutorials/05_simple_system.md)
- [Integrated HVAC user test](tutorials/06_integrated_hvac.md)
- [Curated user documentation source](user/README.md)

## Reference

- [CLI specification](specs/cli.md)
- [Run command reference](reference/cli_run.md)
- [Standalone package reference](reference/standalone_package.md)
- [LSP snapshot reference](reference/lsp_snapshot.md)
- [Kernel plan reference](reference/kernel_plan.md)
- [JIT benchmark harness reference](reference/jit_benchmark.md)
- [v8/v9 language policy](specs/language-v8.md)
- [Fast assignment guide](language/fast_assignment.md)
- [Dimensionless policy guide](language/dimensionless.md)

## Planning and Release

- [Roadmap](roadmap.md)
- [Current master plan pointer](master-plan/current.md)
- [Release acceptance checklist](release/acceptance-checklist.md)
- [Release workflow](release/release-workflow.md)
- [v0.1-preview release notes](release/v0.1-preview.md)
- [v0.2-preview release notes](release/v0.2-preview.md)
- [v0.3-preview release notes](release/v0.3-preview.md)
- [v0.4-preview release notes](release/v0.4-preview.md)
- [v0.5-preview release notes](release/v0.5-preview.md)
- [v0.6-preview release notes](release/v0.6-preview.md)
- [v0.7-alpha release notes](release/v0.7-alpha.md)
- [v0.8-alpha release notes](release/v0.8-alpha.md)
- [v0.9-alpha release notes](release/v0.9-alpha.md)
- [v1.0-stable release notes](release/v1.0-stable.md)
- [v1.0.1 release notes](release/v1.0.1.md)
- [v1.0.2 release notes](release/v1.0.2.md)
- [v1.0.3 release notes draft](release/v1.0.3.md)
- [v9 master plan](master-plan/EngLang_LongTerm_Development_Master_Plan_v9.md)

## Documentation Rules

- Public behavior changes must update README, CLI docs, examples, and release notes.
- Runtime artifact changes must update [Runtime artifacts](architecture/01_runtime_artifacts.md).
- Bytecode/result changes must update [Bytecode VM and result v1](runtime/bytecode.md).
- Core path changes must not add Python or interpreter dependencies.
- Portable release packages should ship curated user PDFs, not the full
  developer markdown tree.
