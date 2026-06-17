# Solver-Centered Plan

This document maps the solver-centered implementation checklist into repository
tracking terms. It is an internal planning map, not a public release contract.
The `v1.1` through `v2.0` labels below are checklist ladder names only; public
release naming still follows [version_plan.md](version_plan.md).

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
| SOLVER-103 | Multi-state RHS evaluator | Internal seed in fixed-step/state-space paths; general equation solving remains planned. |
| SOLVER-104 | Explicit Euler multi-state solver | Internal seed for supported multi-state/state-space paths. |
| SOLVER-105 | RK4 multi-state solver | Internal seed for supported multi-state/state-space paths. |
| SOLVER-106 | Solver diagnostics and failure artifacts | Implemented across supported system/component solver artifacts. |
| SOLVER-201 | StateVector/InputVector parser and semantic | Implemented for current state-space seed scope. |
| SOLVER-202 | LinearOperator unit checking | Implemented for current dense/named-entry operator seed scope. |
| SOLVER-203 | Discrete state-space runtime | Implemented as an internal runtime seed. |
| SOLVER-204 | Continuous state-space via RHS evaluator | Implemented as an internal runtime seed. |
| SOLVER-205 | State-space IDE/report panel | Implemented in report artifacts and IDE smoke. |
| SOLVER-301 | Component instance and port graph collection | Implemented as component graph metadata. |
| SOLVER-302 | Thermal domain connection equations | Implemented for current Thermal assembly fixtures. |
| SOLVER-303 | Equation/unknown classification | Implemented in assembly artifacts. |
| SOLVER-304 | ResidualGraph artifact | Implemented with dependency and solver-plan metadata. |
| SOLVER-305 | ResidualEvaluator interpreter | Implemented as structured residual evaluation and solver-kernel seed paths. |
| SOLVER-401 | Linear algebraic solver | Implemented for square residual graph solves. |
| SOLVER-402 | Fixed-point solver | Implemented as solver-API algorithm seed. |
| SOLVER-403 | Dynamic component fixed-step solver | Implemented as an internal explicit/semi-implicit component seed. |
| SOLVER-404 | Nonlinear Newton seed | Implemented as standalone solver-API seed and covered by CLI smoke for convergence, supplied Jacobian use, and nonconvergence failure artifacts. |
| SOLVER-405 | DAE implicit Euler seed | Implemented as standalone solver-API seed and covered by CLI smoke for state/algebraic convergence, mass-matrix use, inconsistent initial conditions, and timestep nonconvergence artifacts. |
| SOLVER-501 | Delay history buffer | Implemented as solver-API behavior seed. |
| SOLVER-502 | Predictor behavior contract | Implemented as solver-API behavior seed. |
| SOLVER-503 | External behavior wrapper | Implemented as solver-API behavior seed. |
| SOLVER-504 | Behavior node report/IDE | Implemented for delay, Predictor, and external behavior metadata. |

## Checklist Ladder

| Checklist label | Required scope | Current repository status |
| --- | --- | --- |
| v1.0.1 cleanup | Correct solver wording, move metadata-only features to Internal, format examples | Covered by current status, maturity, stable-core, and release-note wording. |
| v1.1 real dynamic system I/O | Solver module, one-state runner behind Solver API, real TimeSeries input/output | Implemented for the supported one-state thermal workflow. |
| v1.2 multi-state explicit ODE | Multi-state RHS evaluator, Euler/RK4, trajectories | Implemented as internal fixed-step/state-space seeds; broad general equation solving remains planned. |
| v1.3 state-space actual simulation | LinearOperator checks, discrete solve, continuous RHS | Implemented as internal state-space seed scope. |
| v1.4 equation assembly | Component graph assembly, generated equations, residual graph | Implemented as internal domain/component assembly seed scope. |
| v1.5 algebraic solver | Linear algebraic solve, fixed-point solve, diagnostics | Implemented as solver-API algorithm seeds and square residual graph solve path. |
| v1.6 small dynamic component solver | Assembled dynamic component graph solves, state/algebraic TimeSeries outputs | Implemented as internal dynamic-component solver seed; not public-supported component graph solving. |
| v1.7 nonlinear/delay/Predictor integration | Nonlinear seed, delay buffer, Predictor contract | Implemented as standalone/API seeds and report/IDE metadata; language-level integration remains planned. |
| v2.0 multi-domain solver | Small multi-domain official example actually solves, report/IDE show equations, plan, residuals, results | Covered by `examples/official/22_multi_domain_boundary_solve` for a constrained Thermal/Fluid/MechanicalNode boundary solve; production multi-domain solving remains planned. |

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
| Official example exercises the solve | Official measured-vs-simulated, multi-state thermal, thermal component assembly, and multi-domain boundary solve examples. |
| Tests cover success and failure | `eng test examples`, cargo tests, `artifacts-check`, `jit-check`, and `ide-check`. |
