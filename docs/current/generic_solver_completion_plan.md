# Generic Solver Completion Plan

This document is the working ledger for turning every solver area currently
called narrow, internal, planned, deferred, or not production-ready into broadly
usable EngLang solver capability. A solver feature is not considered complete
until source syntax, semantic checks, numeric evaluation, failure artifacts,
reports, IDE visibility, examples, tests, and documentation all agree on the
same scope.

## Current Narrow/Not Inventory

| Area | Current implemented scope | Why it is still narrow | Generic end state |
| --- | --- | --- | --- |
| Shared equation-system IR | Several source-system, state-space, and component paths lower into residual-like data structures. | Each lowering path still has shape-specific assumptions and some string-shaped expression handling. | One canonical equation IR for variables, equations, residuals, derivatives, parameters, behavior nodes, events, units, scales, source spans, and domains. |
| Component parameters | Component constructors support empty, named, positional, const, and pure-arithmetic numeric parameter values with unit checks and residual dependencies. | Args/object/non-arithmetic bindings and canonical solver-unit policy are not uniform across every solver path. | Typed constructor values lower into the shared IR and are consumed by linear, nonlinear, dynamic, and DAE solvers with one unit policy. |
| Component equation lowering | Literal boundary seeds, linear port-signal equations, arithmetic residual linearization, parameter constants, and a first unit-parameterized linear coefficient path are implemented. | Expression lowering is still not a full AST/IR pipeline; broad nonlinear, derivative, behavior, affine-unit, and compound-unit equations are not generally supported. | Component equations lower through a typed expression IR preserving units, coefficients, nonlinear operators, derivatives, behavior dependencies, and source spans. |
| Linear algebraic solve | Dense solve works for square constrained residual graphs and exposes residual artifacts. | It is small dense only, with limited scaling and topology analysis. | Linear solves use the shared residual IR with robust scaling, conditioning/failure diagnostics, sparse-ready structure, and broad equation assembly inputs. |
| Fixed-point solve | Source `solve component_graph` works for pivotable linear ResidualGraph loops. | General `x = g(x)` extraction, partitioning, and convergence policy are limited. | Fixed-point mapping extraction works from shared residual IR with relaxation policy, variable scaling, residual history, and nonconvergence artifacts. |
| Nonlinear solve | Newton is wired for narrow coupled multi-variable component residual smokes. | It depends on constrained residual shapes; broad nonlinear component/system expressions are not lowered generically. | Multi-variable nonlinear residual graphs solve with scaling, Jacobian policy, finite-difference fallback, line search/trust-region policy, and source-line residual evidence. |
| DAE solve | Implicit Euler DAE is wired for narrow multi-state unitful temperature examples. | Partitioning, consistent initialization, mass matrices, events, adaptive stepping, and higher-order methods are incomplete. | DAE solve supports robust state/algebraic partitioning, initialization, configurable mass matrix, implicit stepping, events, and failure artifacts. |
| Adaptive ODE solve | One-state thermal and internal continuous state-space use adaptive Heun/Euler with fixed output grid. | Broad multi-state, component-coupled, and event-aware adaptive solving are not supported. | Adaptive ODE works over the shared dynamic IR with TimeSeries interpolation, event hooks, named trajectories, and reproducibility metadata. |
| State-space solve | Typed-block A/B discrete and continuous fixed-step workflows are supported. | Broad operator algebra, nonlinear/DAE composition, discrete adaptive, and component-coupled state-space remain absent. | State-space blocks participate in the shared IR and can compose with nonlinear, DAE, adaptive, and component-coupled solver paths. |
| Behavior graph solve | Delay/Predictor/external identity-wrapper smokes affect narrow explicit-Euler RHS examples. | Model loading, process-backed external execution, nonlinear/DAE coupling, replay policy, and broad behavior expressions are not complete. | Behavior nodes are first-class typed solver graph nodes with deterministic replay, model/process policy, safe/repro enforcement, warnings, and solver coupling. |
| Multi-domain solve | Constrained Thermal/Fluid[Water] pressure-flow residual graphs solve small linear systems. | Physical medium properties, pump/valve/pipe packages, topology diagnostics, energy transport, and nonlinear hydraulic behavior are missing. | Production-oriented domain assembly supports pressure drops, flow resistance, pump/valve seeds, energy transport, medium checks, topology diagnostics, and residual artifacts. |
| IDE/report solver inspection | Many current solver artifacts are visible in report/review/IDE smokes. | Visibility is not yet uniform for every solver failure and every shared-IR entity. | A user can inspect variables, equations, residuals, scales, behavior nodes, dependencies, convergence, and failures without reading raw JSON. |
| Native JIT/AOT | Interpreter kernel IR, benchmark catalog, and fallback metadata exist. | No native code generation or measured speedup claim exists. | Native backend is added only for stable IR subsets with parity tests, benchmark evidence, fallback metadata, and release gates. |

## Cross-Cutting Requirements

Every row above must eventually satisfy these requirements:

1. Source language expresses the model without hidden hard-coded fixtures.
2. Parser/semantic analysis records typed variables, parameters, units, source spans, and diagnostics.
3. Runtime lowers the source model into shared equation/residual/dynamic IR.
4. Numeric evaluation uses the lowered equations or behavior graph, not fabricated values.
5. Success artifacts expose variables, residuals, scales, dependencies, iterations, convergence, and trajectories where applicable.
6. Failure artifacts include a stable code, reason, implicated equation or variable, and source location where available.
7. Report/review/IDE views explain the solve and the failure path.
8. At least one official or internal fixture covers a success path, and diagnostics cover invalid/unsupported paths.
9. `eng test examples`, crate tests, and targeted artifact checks prove the stated scope.
10. Public docs use `Stable`, `Supported`, `Internal`, and `Planned` consistently and do not call narrow smokes generic.

## Implementation Workstreams

### W1. Shared Solver IR

Goal: remove shape-specific bridges by introducing a canonical IR used by source
systems, state-space, and component assemblies.

Tasks:

- Define typed IR entities for `SolverVariable`, `SolverParameter`, `SolverEquation`, `SolverResidual`, `SolverDerivative`, `SolverBehaviorNode`, `SolverEvent`, and `SolverDomainConstraint`.
- Preserve source spans, display unit, canonical unit, quantity kind, scale policy, role, and dependency edges.
- Add adapters from existing system equations, state-space A/B blocks, component assemblies, behavior calls, and domain connection equations.
- Make current residual graph construction consume this IR instead of each frontend rebuilding dependencies independently.
- Snapshot report/review/IDE output for the same model lowered through different frontends.

Evidence gate:

- Unit tests for each frontend-to-IR adapter.
- A mixed fixture whose review artifact shows one dependency model across source equations, component equations, parameters, behavior nodes, and generated connection equations.

### W2. Typed Expression Lowering

Goal: replace string-shape arithmetic parsing with reusable typed expression
lowering.

Tasks:

- Lower numeric literals, unit literals, symbols, unary/binary arithmetic, derivative calls, behavior calls, and supported nonlinear functions into an expression tree.
- Attach dimension and display/canonical unit metadata to every expression node.
- Support coefficient extraction for linear terms, nonlinear residual evaluation, and finite-difference perturbation from the same tree.
- Convert unitful coefficients by numerator/denominator scale rather than relying on raw parameter values.
- Reject unsupported affine or compound-unit forms with precise diagnostics instead of silently producing wrong coefficients.

Evidence gate:

- Linear, unit-parameterized, nonlinear, derivative, and behavior expression fixtures lower through the same API.
- Incompatible unit expressions fail at compile/check time with source-line diagnostics.

### W3. Component Equations and Parameters

Goal: make component-local equations generic enough to feed every solver path.

Tasks:

- Finish unit-parameterized linear equations such as `Q eq UA * (T1 - T2)` across display units.
- Support parameterized dynamic equations and derivative terms through the shared expression tree.
- Support broader constructor values through args/object bindings only after they lower to typed parameter IR.
- Preserve parameter provenance in runtime solution artifacts and report-spec output.
- Add official or internal examples for parameterized linear, nonlinear, and dynamic component equations.

Evidence gate:

- Dense linear, dynamic component, Newton, and DAE tests all consume materialized parameter values from the same parameter IR.
- Examples prove both success and incompatible unit/arity/type diagnostics.

### W4. General Algebraic and Nonlinear Solvers

Goal: turn current dense-linear/fixed-point/Newton smokes into shared residual
solver paths.

Tasks:

- Detect linear, fixed-point-capable, and nonlinear residual graph shapes from shared IR.
- Add variable scale policy, residual scale overrides, and largest-normalized-residual summaries.
- Route supplied analytic/source-linear Jacobian, finite-difference Jacobian, and future JIT Jacobian through one policy.
- Add line-search/trust-region controls, singular/ill-conditioned diagnostics, and nonconvergence artifacts.
- Support coupled unitful variables beyond the current HeatRate smoke.

Evidence gate:

- Success/failure fixtures for multi-variable linear, fixed-point, nonlinear, singular, nonfinite, and nonconvergent cases.
- Report/IDE residual panels show raw and normalized residuals plus source equations.

### W5. General Dynamic and Adaptive Solvers

Goal: make fixed-step/adaptive ODE solving work over source and component-coupled
dynamic IR.

Tasks:

- Build `x`, `z`, `u(t)`, and `p` layouts from shared IR.
- Generate RHS evaluators from derivative equations, residual equations, and behavior nodes.
- Support fixed-step Euler/RK4 and adaptive Heun over multi-state component-coupled systems.
- Add TimeSeries interpolation policy, final partial-step handling, event hooks, and deterministic replay metadata.
- Emit RuntimeTimeSeries for every named state and selected algebraic output.

Evidence gate:

- Official/integration examples for multi-state component dynamics with TimeSeries inputs.
- Adaptive diagnostics include accepted/rejected substeps, event/failure metadata, and fixed output-grid trajectories.

### W6. DAE Completion

Goal: make DAE support a real solver path rather than a narrow implicit-Euler
smoke.

Tasks:

- Infer state/algebraic/parameter/input partition from shared IR.
- Implement consistent initialization with residual evidence and diagnostics.
- Support configurable mass matrices and identity fallback with unit checks.
- Reuse nonlinear solver policy per implicit step.
- Add event and step failure artifacts; keep BDF unsupported until implemented.

Evidence gate:

- Coupled thermal/fluid DAE fixture, mass matrix tests, inconsistent-initial diagnostics, and per-step Newton evidence.

### W7. Behavior Graph Integration

Goal: make delay, Predictor, and external behavior first-class solver nodes.

Tasks:

- Lower behavior calls into typed behavior nodes in shared IR.
- Support delay history, interpolation, and initial-history policy in dynamic and DAE evaluators.
- Add model-loading contracts for Predictor only with provenance/hash and valid-range policy.
- Add process/function external behavior only behind safe/repro profile enforcement and replay metadata.
- Propagate warnings and failures into solver, report, review, and IDE artifacts.

Evidence gate:

- Behavior examples that solve through generic dynamic and DAE paths, not just identity-wrapper explicit RHS smokes.
- Safe/repro tests for blocked or non-deterministic external behavior.

### W8. Production Multi-Domain Seeds

Goal: move beyond constrained Thermal/Fluid linear examples toward useful,
extensible physical domain packages.

Tasks:

- Expand Fluid[Water] with pressure, mass flow, pressure drop, flow resistance, pump, valve, and topology diagnostics.
- Add energy transport coupling between Thermal and Fluid once residual/DAE paths are generic enough.
- Add medium compatibility and parameter range diagnostics.
- Keep examples physically scoped and explicit about limitations until packages mature.

Evidence gate:

- Pressure-flow and thermal-fluid examples with residual/convergence artifacts, singular/topology diagnostics, medium mismatch tests, and report/IDE graph inspection.

### W9. IDE/Report Completion

Goal: make solver behavior inspectable without raw JSON.

Tasks:

- Align report/review/IDE schemas around shared IR identifiers.
- Add panels for equations, variables, residuals, scales, behavior nodes, events, trajectories, and failures.
- Provide source navigation for generated and user-authored equations.
- Keep artifact schema changes release-gated.

Evidence gate:

- IDE smoke covers a success and a failure for algebraic, nonlinear, dynamic, DAE, behavior, and multi-domain examples.

### W10. Performance and JIT

Goal: optimize only after correctness and representative workloads exist.

Tasks:

- Profile supported solver examples and record benchmark baselines.
- Lower stable expression/residual/Jacobian/RHS subsets to interpreter kernels first.
- Add native backend only with parity tests and fallback metadata.
- Keep speedup claims out of docs until benchmark evidence exists.

Evidence gate:

- Interpreter/native parity tests, benchmark catalog, backend choice metadata, and release-gate performance evidence.

## Active Implementation Queue

1. Finish W3 unit-parameterized linear component equations across display units.
   - Compiler: dimension-aware component equation checks.
   - Runtime: coefficient-unit conversion for residual numerator units such as `W/K -> kW per K`.
   - Tests: compiler acceptance, residual coefficient conversion, source-to-runtime solve.
   - Docs: status/matrix/plan wording must state this is a real linear slice, not broad equation support.
2. Promote this slice into an official or compatibility example only after the full example smoke passes.
3. Start W2 typed expression lowering by extracting the current arithmetic linearizer into a typed node pipeline with unit metadata.
4. Move static residual graph and dynamic component residual graph construction onto the same expression-lowering API.
5. Use that shared expression API to broaden nonlinear and DAE source residuals without adding one-off string parsing.

## Completed Slices Within This Plan

- Pressure/Pa and kPa quantity/unit support for the constrained Thermal/Fluid pressure-flow example.
- Component parameter constructors for numeric literal, importable const, positional/named override, and pure arithmetic expression values.
- Parameter values preserved into dense linear, fixed-point, Newton, and DAE residual paths for supported shapes.

## Done Criteria

The goal is complete only when every row in the Current Narrow/Not Inventory has
been moved to the generic end state and verified by current evidence. Passing a
single official example or a narrow smoke does not prove generic completion.
Each completed area must have:

- source syntax and semantic diagnostics;
- shared IR lowering;
- numeric runtime evaluation;
- success and failure artifacts;
- report/review/IDE inspection;
- official or internal fixtures;
- automated tests and example smoke coverage;
- documentation that matches the actual scope.
