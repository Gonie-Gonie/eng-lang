# EngLang Documentation

This index maps the v9 master plan to the concrete repository work.

## Start Here

1. [Getting started](development/00_getting_started.md)
2. [Repository layout](development/01_repo_layout.md)
3. [Daily workflow](development/02_daily_workflow.md)
4. [Reproducible environment policy](development/03_environment_reproducibility.md)
5. [Version roadmap workflow](development/04_version_roadmap_workflow.md)
6. [v1.0 gap audit and hardening register](development/05_v1_0_gap_audit.md)

## Architecture

- [System overview](architecture/00_system_overview.md)
- [Runtime artifacts](architecture/01_runtime_artifacts.md)
- [Compiler frontend](architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](architecture/03_expected_types_and_quantities.md)
- [Data boundary and CSV promote](architecture/04_data_boundary.md)
- [Bytecode VM and result v1](runtime/bytecode.md)
- [TimeSeries statistics guide](guide/timeseries_statistics.md)
- [Plotting guide](guide/plotting.md)
- [Native tester IDE](guide/native_ide.md)
- [Report and review artifacts](guide/report_review.md)
- [Simple system tutorial](tutorials/05_simple_system.md)
- [Integrated HVAC user test](tutorials/06_integrated_hvac.md)

## Reference

- [CLI specification](specs/cli.md)
- [Run command reference](reference/cli_run.md)
- [v8/v9 language policy](specs/language-v8.md)
- [Fast assignment guide](language/fast_assignment.md)
- [Dimensionless policy guide](language/dimensionless.md)

## Planning and Release

- [Roadmap](roadmap.md)
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
- [v8 to v9 revision guide](master-plan/EngLang_v8_to_v9_Revision_Guide.md)
- [v9 master plan](master-plan/EngLang_LongTerm_Development_Master_Plan_v9.md)
- [v8 master plan](master-plan/EngLang_LongTerm_Development_Master_Plan_v8.md)

## Documentation Rules

- Public behavior changes must update README, CLI docs, examples, and release notes.
- Runtime artifact changes must update [Runtime artifacts](architecture/01_runtime_artifacts.md).
- Bytecode/result changes must update [Bytecode VM and result v1](runtime/bytecode.md).
- Core path changes must not add Python or interpreter dependencies.
