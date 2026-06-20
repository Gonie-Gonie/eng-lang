# Generic Solver Completion Plan

This document collects the solver areas that are still described as narrow,
planned, internal, or not production-ready in the current repository. It is the
work plan for turning those scoped solver seeds into broadly usable solver
capability. The work is not complete until each phase below has code, public or
internal examples, artifacts, diagnostics, tests, and documentation that match
the claimed scope.

## Current Gap Summary

| Area | Current narrow support | Generic requirement |
| --- | --- | --- |
| Equation-system IR | Several source/component paths lower into residual graphs, but each path carries shape-specific assumptions. | A shared equation IR for variables, equations, residuals, units, derivatives, parameters, behavior nodes, events, and domains. |
| Component parameters | System-local component instances support empty constructors, named literal argument substitution into component-local boundary/equation seeds, and review/report-spec constructor provenance. | Constructor declarations, defaults, typed parameter binding, unit conversion, and solver-wide parameter metadata must be broad and typed. |
| Component equations | Linear port-signal equations and literal boundary seeds cover the public component solve path. | Component-local equations must support unit-parameterized expressions, nonlinear terms, derivative terms, and robust dependency extraction. |
| Algebraic nonlinear solve | Newton is bridged only for narrow dimensionless source residuals. | Multi-variable nonlinear residual graphs with scaling, Jacobian policy, line search or trust region, diagnostics, and failure artifacts. |
| Fixed-point solve | Supported for pivotable linear residual graph seeds. | General fixed-point mapping extraction, relaxation policy, convergence diagnostics, and nonconvergence artifacts. |
| DAE solve | Small scalar implicit-Euler DAE bridge with identity/mass-matrix fallback. | State/algebraic partitioning, consistent initialization, mass matrix support, events, adaptive stepping, and higher-order methods. |
| Adaptive ODE solve | One-state thermal and internal continuous state-space paths. | Broad multi-state adaptive ODE execution with component-coupled RHS and TimeSeries input interpolation. |
| State-space solve | Typed-block discrete/continuous A/B operator workflows. | Broader operator algebra, nonlinear coupling, DAE/state-space composition, and adaptive discrete constraints. |
| Behavior graph solve | Delay, Predictor, and external wrappers are narrow deterministic source RHS smokes. | First-class behavior nodes with contracts, model loading, external process policy, replayable execution, and solver coupling. |
| Multi-domain solve | Constrained Thermal/Fluid algebraic residual graphs solve simple dense linear systems. | Production-oriented domain assembly with Pressure/Pa, pressure drops, flow resistance, pump/valve seeds, energy transport, medium properties, and topology diagnostics. |
| Native JIT | Benchmark catalog and interpreter fallback are exposed. | Native JIT/AOT should be implemented only after representative solver workloads justify it and artifacts explain backend choice. |

## Completion Phases

| Phase | Deliverables | Evidence gate |
| --- | --- | --- |
| 1. Shared solver IR | One canonical residual/equation IR used by source systems, state-space, and component assemblies. Include variable roles, units, scales, source spans, parameters, derivatives, and behavior dependencies. | Unit tests for IR lowering; result/review/report snapshots show the same dependency model across all supported frontends. |
| 2. Quantity and domain foundations | Fill missing public domain quantities before adding physical examples. Pressure/Pa is the first required slice, followed by domain-specific canonical unit policies and conversion checks. | Compiler quantity/unit tests, IDE completion count smoke, official examples, and runtime residual scale tests. |
| 3. Parameterized components | Support typed constructor declarations, defaults, unit conversion, artifact serialization, and diagnostics for bad arity/type/unit. | Official parameterized component example plus diagnostics for missing/extra/incompatible arguments. |
| 4. General component equation lowering | Replace simple string-shape parsing with expression lowering that preserves units, coefficients, literals, nonlinear operators, derivative terms, and parameter references. | Linear, nonlinear, and dynamic component fixtures all lower through the same expression pipeline; unsupported expressions fail with precise diagnostics. |
| 5. General algebraic solvers | Dense linear remains the baseline; add general multi-variable Newton with scaling, Jacobian policy, finite-difference fallback, line search, singular/nonconvergence artifacts, and largest-residual reporting. | Success/failure fixtures for coupled nonlinear residual graphs with unitful variables and source-line residual evidence. |
| 6. General dynamic solvers | Extend fixed-step and adaptive solvers to component-coupled multi-state systems; preserve named trajectories, interpolation policy, event hooks, and reproducibility metadata. | Official multi-state/component dynamic example, adaptive diagnostics, event boundary tests, and IDE trajectory panels. |
| 7. DAE completion | Implement robust state/algebraic partitioning, consistent initialization, configurable mass matrices, implicit stepping, and DAE failure artifacts. | Coupled thermal/fluid DAE example, inconsistent-initial diagnostics, mass matrix tests, and per-step nonlinear solve evidence. |
| 8. Behavior integration | Make delay, Predictor, and external behavior nodes first-class solver graph nodes with typed contracts, deterministic replay, model/process loading policy, and safe-profile enforcement. | Behavior graph examples that solve through the generic dynamic/DAE pipeline, plus safe/repro profile tests. |
| 9. Production multi-domain seeds | Add pressure-drop, flow resistance, pump/valve, and heat transport examples on top of the generic residual pipeline; keep limitations explicit until full physical modeling is covered. | Official pressure-based Fluid examples, singular/topology diagnostics, medium mismatch tests, and artifact snapshots. |
| 10. Performance and JIT | Profile representative solver examples, then add native backend support only for stable IR subsets with clear fallback and parity tests. | Benchmark catalog, interpreter/native parity tests, backend metadata, and release-gate performance evidence. |

## First Implementation Slice

The first slice is Pressure/Pa support for Fluid examples:

1. Add `Pressure` to the quantity registry with canonical unit `Pa`.
2. Add `Pa` and `kPa` to the unit registry and runtime conversion helpers.
3. Update the constrained Thermal/Fluid official example to use `Fluid[Water]`
   pressure (`p`) and a simple pipe pressure-drop equation instead of
   hydraulic head.
4. Update CLI smoke checks and documentation so the example no longer claims
   `Pressure`/`Pa` is absent.
5. Keep the example's limitation honest: it is a real pressure-based linear
   residual solve, not yet a production hydraulic network model.

## Done Criteria

Each phase is done only when all of these are true:

- The solver computes numeric results from the lowered equations or behavior
  graph.
- Report, review, result, and IDE artifacts expose variables, residuals,
  scales, dependencies, convergence status, and failures.
- At least one official or internal fixture covers the successful path.
- Diagnostics cover representative unsupported or invalid paths.
- The current status, maturity matrix, user docs, examples, and release-facing
  wording match the implemented scope.
