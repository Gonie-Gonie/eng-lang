# Solver-Centered Plan

This document maps the solver-centered implementation checklist into repository
tracking terms. It is an internal planning map, not a public release contract.
The `v1.1` through `v2.0` labels below are checklist ladder names only; public
release naming still follows [version_plan.md](version_plan.md).
For the work needed to turn the remaining narrow solver seeds into generic
solver capability, see
[generic_solver_completion_plan.md](generic_solver_completion_plan.md).

## Regression Gate Evidence

The checklist's "already done" items are guarded through the development gates
below. The latest local `dev.bat release-check` run covered these paths end to
end:

| Checklist area | Gate evidence |
| --- | --- |
| Core execution and packaging | `eng run examples/official/01_csv_plot/main.eng --save-artifacts`, measured-vs-simulated repro run, standalone build, package smoke, extracted portable package smoke, path with spaces, and Korean path smoke. |
| Data boundary, TimeSeries, and report | `artifacts-check` snapshots promoted source hashes, TimeSeries axes, HeatRate-to-Energy integration metadata, measured-vs-simulated two-series plot data, RMSE `TemperatureDelta` metadata, validation results, and report/review/result contracts. |
| General scripting side effects | IDE and artifact smokes cover output manifests, run logs, process results, test results, profile diagnostics, and side-effect artifact panels. |
| Solver baseline | `eng test examples`, runtime cargo tests, `artifacts-check`, `jit-check`, and `ide-check` exercise supported system solver artifacts, state-space seeds, component assembly/residual paths, adaptive substep diagnostics, and solver/report/IDE projection. |
| Distribution readiness | `release-check` runs CI, docs, IDE, artifact, JIT, package, checksum, manifest, and portable package checks before preparing the release manifest. |

## Issue Map

| ID | Scope | Repository disposition |
| --- | --- | --- |
| SOLVER-001 | Extract solver module skeleton | Implemented in `eng_runtime::solver`. |
| SOLVER-002 | Wrap one-state runner behind SolverInput/SolverOutput | Implemented for the supported one-state thermal path. |
| SOLVER-003 | Define TimeGrid/StateLayout/InputLayout | Implemented as solver layout/time-grid contracts. |
| SOLVER-004 | Generate SolverResult and convert to RuntimeSystemSolution | Implemented through runtime artifact adapters. |
| SOLVER-005 | Add solver artifact snapshots | Covered by artifact and CLI smoke checks. |
| SOLVER-101 | TimeSeries system input contract | Implemented for explicit supported TimeSeries thermal inputs. |
| SOLVER-102 | Simulate command binding to SolverInput | Implemented for supported simulation workflows. |
| SOLVER-103 | Multi-state RHS evaluator | Implemented for the supported two-state source-equation fixed-step ODE shape and for internal state-space paths; broad general equation solving remains planned. |
| SOLVER-104 | Explicit Euler multi-state solver | Implemented for the supported two-state source-equation fixed-step ODE shape and internal state-space paths, with CLI/runtime coverage for two-state trajectories, interval-start RHS sampling, final partial steps, and failure artifacts. |
| SOLVER-105 | RK4 multi-state solver | Implemented for the supported two-state source-equation fixed-step ODE shape and internal state-space paths, with CLI/runtime coverage for two-state trajectories, final partial steps, and failure artifacts. |
| SOLVER-106 | Solver diagnostics and failure artifacts | Implemented across supported system/component solver artifacts. |
| SOLVER-107 | Adaptive Heun simulation paths | Implemented for the one-state thermal `simulate` path and internal continuous state-space path as `solver = adaptive_heun`, preserving a fixed output TimeGrid while adapting internal substeps and exposing substep diagnostics in result/report/review/IDE artifacts. |
| SOLVER-201 | StateVector/InputVector parser and semantic | Implemented for supported typed-block state-space syntax plus legacy/internal vector declarations. |
| SOLVER-202 | LinearOperator unit checking | Implemented for supported typed-block A/B operators and current dense/named-entry operator seed scope. |
| SOLVER-203 | Discrete state-space runtime | Implemented for `examples/official/21_state_space_discrete` and internal runtime seeds. |
| SOLVER-204 | Continuous state-space via RHS evaluator | Implemented for `examples/official/22_state_space_continuous` and internal runtime seeds. |
| SOLVER-205 | State-space IDE/report panel | Implemented in report artifacts and IDE smoke. |
| SOLVER-301 | Component instance and port graph collection | Implemented for top-level component fixtures and supported system-local `name = Component(...)` instances with empty constructors or declared numeric parameter defaults/overrides. |
| SOLVER-302 | Thermal domain connection equations | Implemented for the supported `examples/official/23_thermal_component_assembly` boundary graph, source-to-solver `examples/official/24_linear_algebraic_thermal_node`, and internal fixtures. |
| SOLVER-303 | Equation/unknown classification | Implemented in assembly artifacts. |
| SOLVER-304 | ResidualGraph artifact | Implemented with dependency and solver-plan metadata. |
| SOLVER-305 | ResidualEvaluator interpreter | Implemented as structured residual evaluation and solver-kernel seed paths. |
| SOLVER-401 | Linear algebraic solver | Implemented for square residual graph solves and covered by CLI smoke for convergence and singular failure artifacts. |
| SOLVER-402 | Fixed-point solver | Implemented as solver-API algorithm seed and covered by CLI smoke for convergence and nonconvergence failure artifacts. |
| SOLVER-403 | Dynamic component fixed-step solver | Implemented as solver-API explicit/semi-implicit component seeds plus a narrow source `solve component_graph` path for simple-linear dynamic component assemblies. |
| SOLVER-404 | Nonlinear Newton seed | Implemented as standalone solver-API seed and as a narrow source `solve component_graph` bridge for coupled unitful nonlinear residuals; covered by CLI smoke for convergence, supplied/source-linear Jacobian use, residual history, largest residuals, and nonconvergence failure artifacts. |
| SOLVER-405 | DAE implicit Euler seed | Implemented as standalone solver-API seed and as a narrow source `solve component_graph` bridge for small scalar component DAEs; covered by CLI smoke for state/algebraic convergence, algebraic initialization, identity/mass-matrix policy, inconsistent initial conditions, trajectories, and step diagnostics. |
| SOLVER-501 | Delay history buffer | Implemented as solver-API behavior seed and covered by CLI smoke for interpolation plus history-underflow failure artifacts. |
| SOLVER-502 | Predictor behavior contract | Implemented as solver-API behavior seed and covered by CLI smoke for valid-range warnings, contract metadata, and output-layout failure artifacts. |
| SOLVER-503 | External behavior wrapper | Implemented as solver-API behavior seed and covered by CLI smoke for deterministic repro execution, safe-profile rejection, and adapter failure wrapping. |
| SOLVER-504 | Behavior node report/IDE | Implemented for delay, Predictor, and external behavior metadata, including inferred contract fields and diagnostic channels; narrow unitful temperature explicit-Euler source behavior RHS smokes now mark behavior nodes integrated in runtime/report/IDE artifacts. |

## Checklist Ladder

| Checklist label | Required scope | Current repository status |
| --- | --- | --- |
| Claim cleanup | Correct solver wording, move metadata-only features to Internal, format examples | Covered by current status, maturity, package-scope, and release-state wording. |
| Solver API integration | Solver module, one-state runner behind Solver API, real TimeSeries input/output | Implemented for the supported one-state thermal workflow. |
| Multi-state explicit ODE | Multi-state RHS evaluator, Euler/RK4, trajectories | Implemented for the supported two-state source-equation fixed-step ODE shape and internal state-space seeds; broad general equation solving remains planned. |
| State-space actual simulation | LinearOperator checks, discrete solve, continuous RHS | Implemented for supported typed-block discrete/continuous fixed-step workflows plus internal state-space seed scope. |
| Equation assembly | Component graph assembly, generated equations, residual graph | Implemented for the supported constrained Thermal boundary assembly plus broader internal domain/component assembly seeds. |
| Algebraic solver | Linear algebraic solve, fixed-point solve, diagnostics | Implemented as solver-API algorithm seeds, source square residual graph dense linear solve path, and narrow source `solve component_graph` fixed-point path for pivotable linear ResidualGraphs. |
| Small dynamic component solver | Assembled dynamic component graph solves, state/algebraic TimeSeries outputs | Implemented for simple-linear component assemblies through `solve component_graph` with `solver = dynamic_component_explicit_euler` or `solver = dynamic_component_semi_implicit_euler`; covered by `examples/official/26_dynamic_component_room` and runtime/diagnostic fixtures. |
| Nonlinear/delay/Predictor integration | Nonlinear seed, delay buffer, Predictor contract | Adaptive Heun is wired into the one-state thermal path and internal continuous state-space path; narrow source Newton/implicit-Euler DAE bridges are wired into component residual assemblies; narrow unitful temperature explicit-Euler source behavior RHS integration is wired for delay, deterministic Predictor identity-wrapper, and deterministic external adapter identity-wrapper smokes; broader language-level behavior integration remains planned. |
| Small multi-domain solver | Small multi-domain fixture actually solves, report/IDE show equations, plan, residuals, results | Covered by `examples/official/32_small_thermal_fluid_loop` for a constrained Thermal/Fluid[Water] pressure/flow algebraic residual solve and by `examples/internal/22_multi_domain_boundary_solve` for Thermal/Fluid/MechanicalNode boundary coverage; production multi-domain solving remains planned. |

## Final Solver Rule

Solver work is considered complete only when these checks are true for the
specific supported scope being claimed:

| Rule | Evidence path |
| --- | --- |
| Real numeric evaluation happens | Runtime solver APIs, supported system/state-space/component examples, and solver tests. |
| Results are derived from equations or behavior graph, not fabricated | Compiler assembly artifacts, residual graph metadata, and runtime adapter tests. |
| State, algebraic, input, and output variables are named and typed | Solver layout contracts and report/IDE solver inspectors. |
| Residuals are computed and inspectable | Component solver residuals, normalized residuals, largest residuals, and residual dependency graph inspectors. |
| Failure is reported with reason | Failure artifacts, `failure_code`, `failure_reason`, and convergence status fields. |
| TimeSeries outputs are generated | Solver trajectories are converted into TimeSeries-style report/result/IDE rows. |
| Report/review artifacts explain the solve | `report_spec.json`, `review.json`, `.engres`, and HTML report solver sections. |
| IDE shows the solve | `dev.bat ide-check` smoke covers solver, residual, dependency, behavior, state-space, and kernel inspectors. |
| Example or fixture exercises the solve | Official measured-vs-simulated covers the supported one-state thermal workflow; `examples/official/20_multi_state_thermal` covers the supported two-state source-equation fixed-step ODE workflow; `examples/official/21_state_space_discrete` and `examples/official/22_state_space_continuous` cover supported typed-block state-space workflows; `examples/official/23_thermal_component_assembly` covers the supported constrained Thermal component boundary assembly; `examples/official/24_linear_algebraic_thermal_node` covers the source-to-solver linear algebraic ResidualGraph solve; `examples/official/25_fixed_point_loop` covers the source fixed-point ResidualGraph solve; `examples/official/26_dynamic_component_room` covers the simple-linear dynamic component source solve; `examples/official/27_nonlinear_algebraic` covers the source Newton residual solve; `examples/official/28_small_dae` covers the source multi-state unitful implicit-Euler DAE residual solve; `examples/official/29_delay_component_solver`, `examples/official/30_predictor_component_solver`, and `examples/official/31_external_behavior_solver` cover narrow unitful temperature source behavior RHS solves; `examples/official/32_small_thermal_fluid_loop` covers a constrained Thermal/Fluid[Water] pressure/flow algebraic residual solve. Internal fixtures cover state-space thermal and additional constrained multi-domain boundary solve seeds. |
| Tests cover success and failure | `eng test examples`, cargo tests, `artifacts-check`, `jit-check`, and `ide-check`. |
