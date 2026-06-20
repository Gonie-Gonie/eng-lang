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
| Adaptive ODE solve | Source-equation, one-state thermal, and internal continuous state-space paths use adaptive Heun/Euler with fixed output grids. | Component-coupled, event-aware, and production adaptive solving are not supported. | Adaptive ODE works over the shared dynamic IR with TimeSeries interpolation, event hooks, named trajectories, and reproducibility metadata. |
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


## Narrow/Not Claim Closure Matrix

This matrix turns the current `narrow`, `internal`, `planned`, `deferred`, and
`not production` solver statements into concrete closure gates. A row should be
removed from public limitation text only after its evidence gate is satisfied by
current tests and artifacts.

| Current narrow/not claim | Generic implementation work required | Evidence gate before claim can be removed | First aligned slices |
| --- | --- | --- | --- |
| General equation-system runtime is beyond the supported one/two-state source-equation shapes. | Lower arbitrary checked system equations into the shared solver IR, build named `x`, `u(t)`, `p`, `z`, output, and derivative layouts, and evaluate RHS/residual expressions through the typed expression tree. | Official multi-state non-thermal source example plus diagnostics for missing/duplicate derivatives, unsupported algebraic loops, unit mismatches, non-finite RHS, and generated RuntimeTimeSeries for every state/output. | Promote current two-state RHS evaluator into a shared system-IR adapter; add source spans and unit metadata to every lowered derivative equation. |
| Broad adaptive solving is not supported beyond source-equation, one-state thermal, and internal continuous state-space paths. | Run adaptive Heun over the shared dynamic IR, support component-coupled RHS evaluators, add interpolation and event hooks, and preserve accepted/rejected substeps per output sample. | Component-coupled adaptive fixtures expose fixed output-grid trajectories, substep diagnostics, event/failure artifacts, and report/IDE panels. | Reuse `TimeGrid`/adaptive step reports from source-equation, one-state thermal, and state-space paths, then swap RHS source to shared dynamic IR. |
| State-space is limited to typed-block A/B fixed-step workflows. | Lower state-space operators into the shared IR, support operator algebra subsets, discrete adaptive policy, and composition with nonlinear/DAE/component residual paths. | State-space examples can be solved through the same dynamic/residual artifact path as source systems, with operator diagnostics and shared residual/RHS IDs in review/IDE. | Add adapter from `StateSpaceRhsEvaluator` matrices to shared solver IR variables/equations. |
| Component equations are limited to literal seeds, simple linear forms, and selected unit-parameterized coefficients. | Lower component-local arithmetic, derivative, behavior, nonlinear functions, affine units, compound units, and parameter values through one typed expression API. | Linear, dynamic, nonlinear, and DAE component examples all consume the same expression tree; unsupported unit algebra fails with source diagnostics instead of string-shape fallback. | Continue replacing one-off residual parsing with reusable expression AST metadata and evaluator entrypoints. |
| Broad args/object/non-arithmetic constructor bindings are not supported. | Introduce typed constructor-value IR for literals, imports, object fields, args values, and computed expressions with canonical unit conversion and provenance. | Dense linear, fixed-point, Newton, DAE, and dynamic component tests consume constructor values from the same parameter IR; report/review show provenance and rejected arity/type/unit cases. | Extend existing numeric/importable/pure-arithmetic parameter materialization into a shared `SolverParameter` adapter. |
| Fixed-point source solve is narrow and pivotable-linear only. | Detect general `x = g(x)` partitions from shared residual IR, add variable scaling, relaxation policy, residual history, and nonconvergence artifacts. | Success/failure fixtures cover coupled fixed-point loops, bad partitions, non-finite updates, tolerance/max-iteration options, and report/IDE residual history. | Move fixed-point update extraction from residual-shape helper to shared residual IR metadata. |
| Newton source solve is a narrow coupled component residual smoke. | Evaluate broad nonlinear residual trees from shared IR, add variable scales, Jacobian policy, line-search/trust-region diagnostics, finite-difference fallback, and source-line residual evidence. | Multi-variable unitful nonlinear source/component examples with nonlinear operators, singular/ill-conditioned/nonconvergent cases, and report/IDE Jacobian/residual diagnostics. | Use parsed residual ASTs for all Newton evaluations and surface inferred expression units in solver artifacts. |
| Implicit-Euler DAE is a narrow temperature example, not a broad DAE solver. | Infer robust state/algebraic/input/parameter partitions, initialize consistently with residual evidence, support configured mass matrices, event/failure artifacts, and keep unsupported methods explicit. | Coupled thermal/fluid DAE fixture plus inconsistent initial, mass-matrix, per-step Newton, and unsupported BDF diagnostics visible in artifacts. | Reuse shared residual expression ASTs for DAE residual evaluation and extend partition diagnostics. |
| Dynamic component solving is simple-linear and fixed-step only. | Build dynamic component layouts from shared IR, support algebraic-free and semi-implicit paths with parameterized derivatives, selected nonlinear algebraic solves, adaptive stepping, and selected algebraic outputs. | Dynamic component fixtures cover multi-state, parameters, TimeSeries inputs, algebraic trajectories, nonconvergence, and report/IDE timestep diagnostics. | Move dynamic residual graph construction fully onto typed expression lowering and expose inferred units in artifacts. |
| Behavior graph solving is limited to identity-wrapper explicit-Euler RHS smokes. | Lower behavior calls into first-class solver IR nodes with deterministic replay, model loading contracts, process policy, range warnings, finite-difference policy, and dynamic/DAE coupling. | Delay, Predictor, and external examples affect dynamic and DAE solves, safe/repro failures are enforced, and behavior node warnings/failures appear in report/IDE. | Connect behavior-node contracts to shared expression/RHS metadata instead of component-local string scans. |
| Thermal/Fluid multi-domain solve is constrained and not production. | Add domain packages for Fluid[Water] pressure, mass flow, pressure drop, pump/valve/pipe parameters, topology diagnostics, medium checks, and energy transport coupling. | Pressure-flow and thermal-fluid examples solve with residual/convergence artifacts, topology/singular/medium mismatch diagnostics, and report/IDE graph inspection. | Generalize current pressure-flow residual graph into domain package components with checked parameter ranges. |
| IDE/report visibility is not uniform across solver failures and shared-IR entities. | Align runtime result, report-spec, review JSON, and IDE inspectors around shared variable/equation/residual IDs, units, scales, source spans, trajectories, events, and failures. | IDE smoke covers success and failure for algebraic, nonlinear, dynamic, DAE, adaptive, behavior, and multi-domain examples without reading raw JSON. | Expose residual expression inferred units in component solver result artifacts, graph-level residual metadata, and IDE residual panels. |
| Native JIT/AOT has no backend or speed claim. | Keep interpreter kernel IR as the first executable backend, add native backend only after stable IR subset parity tests and benchmark evidence exist. | Benchmarks show correctness parity, selected backend metadata, fallback reason, and measured speedup before any public native claim. | Keep native unavailable in release claims while expanding residual/Jacobian/RHS interpreter kernels. |
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

1. Add graph-level residual metadata entries with expression unit/quantity/source spans once the shared solver IR owns residual identity.
2. Move static residual graph and dynamic component residual graph construction onto the same expression-lowering API.
3. Use that shared expression API to broaden nonlinear and DAE source residuals without adding one-off string parsing.
4. Generalize component-local parameterized dynamic equations and derivative terms through the shared expression tree.
5. Revisit fixed-point, Newton, DAE, and behavior graph source bridges after they consume the shared expression API instead of shape-specific parsing.

## Completed Slices Within This Plan

- Pressure/Pa and kPa quantity/unit support for the constrained Thermal/Fluid pressure-flow example.
- Component parameter constructors for numeric literal, importable const, positional/named override, and pure arithmetic expression values.
- Parameter values preserved into dense linear, fixed-point, Newton, and DAE residual paths for supported shapes.
- Unit-parameterized linear component equations such as `Q eq UA * (T1 - T2)` across compatible display units, with compiler dimension checks, runtime coefficient conversion, source-to-runtime tests, docs, and `examples/official/33_unit_parameterized_wall` example-smoke coverage.
- Runtime arithmetic residual expressions now lower once into a reusable AST used by both evaluation and finite-difference linearization, including derivative-symbol alias reuse across updated symbol values.
- Numeric unit literals in the reusable arithmetic expression tree preserve display unit, canonical unit, and quantity-kind metadata from the existing unit registry without changing existing evaluator conversion behavior.
- Source Newton and implicit-Euler DAE residual loops pre-parse assembly residual expressions into reusable arithmetic ASTs instead of reparsing residual strings on every iteration/sample evaluation.
- Reusable arithmetic expression ASTs now propagate display unit, canonical unit, and quantity-kind metadata from supplied source symbols through unary and binary arithmetic nodes, including derivative aliases and unit-derived products such as Conductance * TemperatureDelta -> Power, without changing numeric evaluator behavior.
- Static and dynamic ResidualGraph construction now consumes propagated expression metadata by storing inferred units on residual expressions, with regression coverage for unit-parameterized Thermal residuals and dynamic derivative residuals.
- Component solver result artifacts now expose residual expression inferred unit and quantity-kind fields through runtime result JSON, report-spec JSON, and IDE residual inspectors.
- Component residual graphs now carry per-residual metadata entries from compiler IR into report-spec JSON, including source expression, residual expression, dependencies, source line, and runtime-enriched unit, expression unit, quantity-kind, scale policy, and status fields.
- Static component residual graph and dynamic component residual graph construction now call the same profile-based linear residual expression lowering entrypoint, with separate failure profiles for static component and dynamic component contexts.
- Source implicit-Euler DAE residual solves now materialize assembly parameter values into DaeInput, algebraic initialization, final residual samples, solver-plan metadata, and report-spec artifacts, with regression coverage for parameterized DAE residuals and explicit sample parameter override.
- Source Newton residual solves now materialize assembly parameter values once, pass explicit parameter vectors through parse/evaluation closures, reject parameter-vector layout mismatches, and cover explicit algebraic parameter override in regression tests.
- Behavior graph explicit-Euler source RHS now pre-parses derivative residual expressions once with state, derivative, input, parameter, and behavior-output symbols, then reuses parsed expressions during timestep evaluation with regression coverage for behavior outputs plus explicit parameter values.
- Behavior graph source outputs now carry unit and quantity metadata into residual expression parsing, preserving inferred residual expression units when behavior-node outputs participate in derivative RHS equations.
- Fixed-point source residual solves now expose solver residual-history diagnostics through runtime component solutions and report-spec artifacts, including final convergence or nonconvergence failure metadata.
- Source DAE solve requests now pass explicit DAE method policy into `DaeOptions`, so unsupported BDF requests surface the solver API `E-DAE-METHOD-UNSUPPORTED` failure through component solution and report-spec artifacts.
- Source-equation ODE simulations now route `solver = adaptive_heun` through the shared `SourceRhsEvaluator` path for generic numeric state layouts, preserving fixed output-grid trajectories and adaptive substep diagnostics.
- Source ODE RHS evaluation now pre-parses derivative coefficients and RHS expressions through the shared arithmetic expression parser, preserving unit-literal metadata and removing per-sample RHS string parsing for fixed-step and adaptive source simulations.
- Source ODE RHS parser construction now receives typed input and parameter symbol metadata from runtime layouts, so source derivative expressions preserve state/input/parameter units instead of name-only RHS symbols.

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
