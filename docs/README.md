# EngLang Documentation

This index maps the current repository status to the concrete documentation
needed for implementation. Start with the short current-status layer before
opening long-form planning or internal solver documents.

## Start Here

1. [Root README](../README.md)
2. [Current project status](current/status.md)
3. [Integrated language philosophy](current/philosophy.md)
4. [Feature maturity matrix](current/feature_maturity_matrix.md)
5. [LLM context](../LLM_CONTEXT.md)

## Current Scope And Planning

- [Version plan](current/version_plan.md)
- [Public package scope](current/stable_core_scope.md)
- [Development tracks](current/tracks.md)
- [Uncertainty and distribution numeric track](current/uncertainty.md)
- [Reviewability as a language feature](current/reviewability.md)
- [Composite workflow base modules](current/workflow_modules.md)
- [Test and CI gate map](current/test_ci_gates.md)
- [Main internal status](current/main_internal_status.md)
- [Implementation issue backlog](current/implementation_issue_backlog.md)
- [LLM load map](llm/load_map.yml)
- [Roadmap](roadmap.md)

## User And Workflow Guides

- [Native tester IDE](guide/native_ide.md)
- [TimeSeries statistics guide](guide/timeseries_statistics.md)
- [Plotting guide](guide/plotting.md)
- [Report and review artifacts](guide/report_review.md)
- [Language grammar guide](guide/language_grammar.md)
- [Data-driven modeling track guide](guide/data_driven_modeling.md)
- [Uncertainty track guide](guide/uncertainty.md)
- [Domain and component track guide](guide/domain_component.md)
- [Class object guide](guide/class_object.md)
- [Curated user documentation source](user/README.md)

## Architecture

- [System overview](architecture/00_system_overview.md)
- [Runtime artifacts](architecture/01_runtime_artifacts.md)
- [Compiler frontend](architecture/02_compiler_frontend.md)
- [Expected types and quantity completions](architecture/03_expected_types_and_quantities.md)
- [Data boundary and CSV promote](architecture/04_data_boundary.md)
- [Bytecode VM and result v1](runtime/bytecode.md)
- [Side effect and general programming policy](reference/side_effect_policy.md)

## Reference

- [CLI specification](specs/cli.md)
- [Run command reference](reference/cli_run.md)
- [Standalone package reference](reference/standalone_package.md)
- [Breaking change policy](reference/breaking_change_policy.md)
- [LSP snapshot reference](reference/lsp_snapshot.md)
- [Kernel plan reference](reference/kernel_plan.md)
- [JIT benchmark harness reference](reference/jit_benchmark.md)
- [v8/v9 language policy](specs/language-v8.md)
- [Fast assignment guide](language/fast_assignment.md)
- [Dimensionless policy guide](language/dimensionless.md)
- [Unit-aware print and CSV summary export](language/string_formatting.md)
- [Artifact schemas](schemas/README.md)

## Advanced / Internal Engineering Tracks

These documents are implementation context, not product identity or public
package scope unless a current status document says so.

- [Solver docs index](solver/README.md)
- [Solver-centered implementation plan](current/solver_centered_plan.md)
- [Generic solver completion plan](current/generic_solver_completion_plan.md)
- [JIT benchmark reference](reference/jit_benchmark.md)
- [Kernel plan reference](reference/kernel_plan.md)

## Development

- [Getting started](development/00_getting_started.md)
- [Repository layout](development/01_repo_layout.md)
- [Daily workflow](development/02_daily_workflow.md)
- [Reproducible environment policy](development/03_environment_reproducibility.md)
- [Version roadmap workflow](development/04_version_roadmap_workflow.md)
- [Historical stable-core gap audit](development/05_historical_stable_core_gap_audit.md)

## Planning And Release

- [Release acceptance checklist](release/acceptance-checklist.md)
- [Release workflow](release/release-workflow.md)
- [Release state audit](release/release-state.md)
- [v0.1.0 release notes](release/v0.1.0.md)

Historical preview release notes live under
[archive/release-notes](archive/release-notes/README.md) for repository
continuity, but they are not first-read public package documents. Historical
long-form plans are reachable through [current planning pointer](master-plan/current.md)
only when old rationale is needed.

## Documentation Rules

- Public behavior changes must update README, current status, maturity matrix,
  examples, and release notes.
- Runtime artifact changes must update [Runtime artifacts](architecture/01_runtime_artifacts.md).
- Bytecode/result changes must update [Bytecode VM and result v1](runtime/bytecode.md).
- Core path changes must not add Python or interpreter dependencies.
- Portable release packages should ship curated user PDFs, not the full
  developer markdown tree.
- Public release labels and future development tracks must remain separate.
- Solver-heavy implementation detail belongs in internal track documents, not
  in first-user docs.
