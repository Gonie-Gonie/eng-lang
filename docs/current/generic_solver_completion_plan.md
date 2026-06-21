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
| DAE solve | Implicit Euler DAE is wired for narrow multi-state unitful temperature examples. | Partitioning, robust initialization, unit-aware mass matrices, events, adaptive stepping, and higher-order methods are incomplete; source-level dimensionless scalar/diagonal/dense mass matrices are implemented. | DAE solve supports robust state/algebraic partitioning, initialization, configurable mass matrix, implicit stepping, events, and failure artifacts. |
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


## Checklist Reconciliation Against Current Main

The two external solver checklists are treated as implementation roadmaps, not
current release claims. Their old `v1.x` ladder language is intentionally not
used as current versioning evidence; the authoritative public line remains
`v0.1.0`, and the active target is solver hardening on `main`.

The checklist items map to the current ledger as follows:

| Checklist theme | Current ledger home | Completion rule |
| --- | --- | --- |
| Solver API, SolverInput, SolverResult, and RuntimeData path | W1, W5, W9 | Every source/system/component solve produces a real `SolverResult` or component solution, and no supported solver output is fabricated outside the solver path. |
| Multi-state source ODE and TimeSeries input/output | W1, W5 | Source equations lower to shared dynamic IR, all named states and outputs materialize as RuntimeTimeSeries, and fixed/adaptive diagnostics are inspectable. |
| State-space actual simulation | W1, W5 | State-space blocks lower into the shared dynamic/residual artifact model instead of remaining a separate typed-block path. |
| Component graph assembly | W1, W2, W3 | Component instances, ports, connections, generated equations, parameters, source spans, and domain constraints are canonical shared IR entries. |
| ResidualGraph, ResidualEvaluator, residual scaling | W2, W4, W9 | Every algebraic/nonlinear/DAE path exposes raw residuals, normalized residuals, scales, largest residual evidence, and failure source context. |
| Algebraic, fixed-point, Newton, and DAE integration | W4, W6 | Solvers consume shared residual IR, not narrow shape-specific bridges, and success/failure fixtures cover broad variable/unit layouts. |
| Dynamic component solve | W5, W6 | Dynamic component layouts come from shared IR, support selected algebraic variables and algebraic-free nonlinear RHS expressions, and expose trajectory plus per-step failure evidence. |
| Delay, Predictor, and external behavior | W7 | Behavior calls become typed solver graph nodes with replay/provenance/profile policy and dynamic/DAE coupling. |
| Production multi-domain packages | W8 | Thermal/Fluid examples move from constrained pressure-flow smokes to checked domain packages with topology, medium, pressure-drop, and energy-coupling evidence. |
| IDE/report solver inspection | W9 | A user can inspect equations, residuals, scales, dependencies, trajectories, behavior nodes, and failures without reading raw JSON. |
| JIT/native optimization | W10 | Native claims require parity tests, benchmark evidence, backend metadata, and fallback reason artifacts. |

Immediate execution order is evidence-driven:

1. Close residual evidence gaps first: per-step largest residual identity,
   normalized residual values, scales, and source equation names in runtime,
   report, review, and IDE artifacts.
2. Move remaining source/component residual evaluators onto the shared typed
   expression tree and remove shape-specific string parsing.
3. Generalize dynamic and DAE layout construction from the same shared IR.
4. Promote behavior nodes from identity-wrapper RHS smokes into typed dynamic and
   DAE evaluator inputs.
5. Only after correctness and artifacts are broad enough, expand multi-domain
   packages and JIT execution.

## Progress Ledger

- 2026-06-21: Source `solve component_graph` Newton and implicit-Euler DAE residual evaluations now accept user-provided residual scale overrides through `with { residual_scale = residual = value unit }` or `with { residual_scales = [...] }`. The source Newton fixture proves overridden residual scales affect source-linear Jacobian scaling, raw/normalized residual vectors, final residual metadata, and report-spec artifacts; the diagnostics fixture proves invalid scales surface explicit `E-SOURCE-RESIDUAL-SCALE` failure artifacts. This advances W4 residual scaling evidence without claiming broad nonlinear/DAE solving or a general scale-policy language.
- 2026-06-21: Component solver step diagnostics now attach source context for the largest named residual by projecting `largest_residual_name` through component residual metadata. Runtime `.engres` and report-spec artifacts expose `largest_residual_source_expression`, `largest_residual_source_line`, and `largest_residual_source_reason` for Newton-style component steps, reducing the need to manually join step diagnostics with residual lists while preserving synthetic diagnostics such as `adaptive_error_norm` as source-less.
- 2026-06-21: Shared arithmetic expression trees now support the broader dimensionless one-argument trig subset `tan`, `asin`, `acos`, and `atan` alongside `sqrt`, `exp`, `ln`, `sin`, and `cos`. Runtime coverage now proves `tan()` in a source Newton residual, `atan()`/`tan()` in source-equation RK4 RHS/output evaluation, and `atan()` in a source implicit-Euler DAE residual, while compiler semantic checks still reject unitful arguments before runtime solving.
- 2026-06-21: `dynamic_component_explicit_euler` and `dynamic_component_semi_implicit_euler` source solves now fall back from affine derivative residual extraction to Newton derivative residual solves for selected parsed dimensionless residuals. Runtime smokes cover explicit and semi-implicit success paths, preserve `newton_converged` residual diagnostics in runtime/report artifacts, and keep fixed-step derivative Newton nonconvergence explicit as `E-NEWTON-NONCONVERGENCE`; this does not claim broad derivative-rich component equations, event handling, or production component simulation.
- 2026-06-21: Fixed-step `dynamic_component_explicit_euler` and `dynamic_component_semi_implicit_euler` derivative-Newton coverage now includes combined TimeSeries component-input fixtures. The runtime smokes prove `drive_data.drive` feeds parsed nonlinear derivative residuals, per-step Newton diagnostics retain `node.equation_1` largest-residual evidence, and trajectories/report artifacts stay aligned without claiming broad component-coupled TimeSeries or derivative-rich equations.
- 2026-06-21: `dynamic_component_adaptive_heun` derivative-Newton coverage now includes a combined TimeSeries component-input fixture. The runtime smoke proves `drive_data.drive` feeds parsed nonlinear derivative residuals while accepted adaptive substep diagnostics, Newton residual diagnostics, and fixed output-grid trajectories remain aligned; this still does not claim broad adaptive component timestepping, events, or production component simulation.
- 2026-06-21: `dynamic_component_adaptive_heun` source solves now reuse adaptive Heun over parsed derivative residual expressions and emit fixed output-grid component trajectories plus accepted-substep diagnostics in runtime/report artifacts. Runtime smoke covers scalar and fixed-step TimeSeries component inputs, selected nonlinear derivative residual Newton fallback with RHS residual diagnostics, selected affine/Newton algebraic output materialization, output-grid Newton residual diagnostics, and the combined TimeSeries-driven Newton algebraic materialization path, while diagnostics fixtures keep Newton derivative/algebraic nonconvergence explicit as `E-NEWTON-NONCONVERGENCE`, so this does not claim broad adaptive component timestepping, broad nonlinear algebraic coupling, behavior coupling, or event support.
- 2026-06-21: Source implicit-Euler DAE residual solves now materialize component input variables from `with { inputs = ... }`, including fixed-step TimeSeries sampling during algebraic initialization, implicit steps, and final residual evaluation. The runtime fixture proves `drive_data.drive` feeds `node.drive` in DAE residual expressions, state/algebraic trajectories, per-step Newton diagnostics, and report artifacts without claiming broad DAE input partitioning, adaptive DAE timestepping, or event support.
- 2026-06-21: Source implicit-Euler DAE residual solves now evaluate typed deterministic `predictor(signal)` and `adapter(signal)` identity-wrapper behavior outputs inside algebraic initialization, implicit-step Newton samples, and final residual evaluation. Delay behavior in DAE now returns explicit `E-BEHAVIOR-SOURCE-DAE-DELAY` failure artifacts because replay-safe delay history mutation during Newton iteration is still not implemented.
- 2026-06-21: `dynamic_component_semi_implicit_euler` now has a combined TimeSeries component-input plus nonlinear algebraic Newton fallback fixture. The runtime fixture proves `drive_data.drive` feeds a parsed component input while `boundary.node.balance * boundary.node.balance eq boundary.node.x + boundary.drive` lowers to a parenthesized residual, converges through per-step Newton diagnostics, and materializes state/algebraic trajectories without claiming broad component-coupled TimeSeries or nonlinear solving.
- 2026-06-21: `dynamic_component_semi_implicit_euler` now has a selected algebraic output trajectory fixture. The runtime fixture proves `node.y eq cos(node.x)` is solved through the semi-implicit Newton algebraic fallback, materializes `node.node.y` and connected algebraic trajectories, and exposes residual/step diagnostics in report artifacts without claiming broad shared-IR output lowering or adaptive component timestepping.
- 2026-06-21: `dynamic_component_semi_implicit_euler` parsed derivative RHS coverage now includes constructor-overridden component parameters. The runtime fixture proves `node.k * sin(node.node.x)` dependencies, constructor provenance, dense linear algebraic step diagnostics, state/algebraic trajectories, and report artifacts are preserved in the semi-implicit path without claiming broad nonlinear algebraic component solving or adaptive component timestepping.
- 2026-06-21: `dynamic_component_semi_implicit_euler` source solves now have a dedicated nonlinear algebraic Newton nonconvergence fixture. The diagnostics smoke proves the per-step Newton fallback surfaces `E-NEWTON-NONCONVERGENCE`, top-level `algebraic_solve_failed`, step-level `newton_not_converged`, normalized residual vectors, and report HTML evidence without claiming broad nonlinear algebraic component solving.
- 2026-06-21: `dynamic_component_semi_implicit_euler` source solves now materialize `with { inputs = TimeSeriesName }` component inputs by sampling RuntimeTimeSeries values through the residual-graph algebraic and RHS evaluators. The runtime fixture proves `heat_data.Q_drive` feeds `boundary.q`, dense linear algebraic step diagnostics, state/algebraic trajectories, report artifacts, and TimeSeries alignment evidence without claiming adaptive component timestepping or broad component-coupled TimeSeries input solving.
- 2026-06-21: `dynamic_component_semi_implicit_euler` source solves now fall back from unsupported linear algebraic residual lowering to per-step Newton algebraic residual solves over parsed source expressions. The runtime fixture proves a dimensionless nonlinear algebraic boundary residual drives the algebraic trajectory, carries `newton_converged` step diagnostics, and keeps state/algebraic trajectories in report artifacts without claiming broad nonlinear algebraic component solving.
- 2026-06-21: `dynamic_component_semi_implicit_euler` source solves now combine a dense linear algebraic residual graph with parsed derivative residual RHS evaluation. The runtime fixture proves a dimensionless `sin(state)` derivative residual drives the semi-implicit state trajectory while algebraic boundary residuals still expose raw/normalized residual vectors and largest-residual diagnostics, without claiming nonlinear algebraic solves or adaptive component timestepping.
- 2026-06-21: Parsed `dynamic_component_explicit_euler` source solves now materialize `with { inputs = TimeSeriesName }` component inputs by sampling RuntimeTimeSeries values at each fixed-step RHS evaluation. The runtime fixture proves `drive_data.drive` feeds `node.drive` dependencies, trajectory output, report artifacts, and TimeSeries alignment evidence without claiming adaptive or broad component-coupled TimeSeries input solving.
- 2026-06-21: Component-local input declarations now lower into component assembly input variables and the dynamic component solver input layout, with `with { inputs = ... }` feeding parsed explicit dynamic RHS evaluation. The runtime fixture proves `node.drive` dependency evidence, input counts, trajectory output, and report artifacts for a scalar component input without claiming component-coupled adaptive solving.
- 2026-06-21: Source-equation adaptive Heun coverage now has a two-state runtime fixture with scalar output materialization, accepted substep diagnostics, report-spec solver results, and PlotSpec series. This strengthens W5 source dynamic evidence without claiming component-coupled adaptive solving, events, or production adaptive policy.
- 2026-06-21: `dynamic_component_explicit_euler` parsed derivative residual coverage now includes a selected algebraic output trajectory fixture, proving `node.y eq cos(node.x)` is carried into runtime/report artifacts alongside the state trajectory and per-step residual evidence. This advances W5 output materialization without claiming general nonlinear algebraic coupling or adaptive component timestepping.
- 2026-06-21: Algebraic-free `dynamic_component_explicit_euler` function RHS coverage now includes a coupled two-state dimensionless `sin`/`cos` residual fixture, proving parsed derivative residual expressions are not limited to one state and that multi-state trajectories, equation counts, dependency metadata, and report artifacts stay aligned.
- 2026-06-21: Algebraic-free `dynamic_component_explicit_euler` function RHS coverage now includes constructor-overridden component parameters in parsed residual expressions, proving `node.k * sin(node.node.x)` dependencies, parameter provenance, trajectory output, and report artifacts are preserved through the explicit dynamic component evaluator.
- 2026-06-21: Algebraic-free `dynamic_component_explicit_euler` source solves now route derivative residuals through parsed arithmetic residual expressions, including a dimensionless `sin()` RHS fixture with runtime, report-spec, report HTML, trajectory, and step-diagnostic smoke coverage. This advances W5 without claiming nonlinear algebraic coupling, adaptive component timestepping, or broad unitful function support.
- 2026-06-20: Source implicit-Euler DAE residual solving now has dimensionless `sin`/`cos` fixture and example-smoke coverage, proving the shared arithmetic function subset is exercised in DAE residual evaluation, step diagnostics, report-spec, and report HTML artifacts.
- 2026-06-20: Source-equation fixed-step RHS simulation now has runtime fixture and example-smoke coverage for dimensionless `sin`/`cos` calls in derivative and algebraic output equations, proving the shared arithmetic function subset is exercised outside Newton residual solves as well.
- 2026-06-20: Component equation semantic checks were extended for the supported dimensionless math-function subset and reject unitful `sqrt`/`exp`/`ln`/`sin`/`cos` arguments before runtime residual solving, with diagnostics smoke coverage. The subset has since expanded to `tan`/`asin`/`acos`/`atan` with the same dimensionless-only policy.
- 2026-06-20: Shared arithmetic expression parsing initially recognized dimensionless one-argument math functions `sqrt`, `exp`, `ln`, `sin`, and `cos`, rejected unitful arguments with solver-profile parse diagnostics, and covered source Newton solving through a dimensionless `sqrt()` residual fixture. This advanced W2/W4 expression reuse without claiming broad unitful nonlinear function support; later slices expanded the same path with additional trig functions.
- 2026-06-20: Newton results now preserve per-iteration residual vectors, and source Newton/DAE component step diagnostics expose the largest residual index/name/value for each iteration or implicit step in report-spec artifacts. This strengthens W4/W9 residual evidence without claiming that broad nonlinear or DAE solving is complete.
- 2026-06-20: Source-equation ODE simulation accepts scalar `output` system variables, evaluates their algebraic expressions from each simulated state/input/parameter sample, stores them as solver algebraic trajectories, and materializes `RuntimeTimeSeries` such as `sim.Q_load`. This closes the state-only output limitation for the supported source-equation ODE path; shared-IR, component-coupled, DAE, and behavior output lowering remain in the generic workstreams.
- 2026-06-20: Typed-block state-space simulation now allows `outputs y = [...]` to include scalar `output` variables, evaluates those output equations from state/input/parameter samples after discrete, fixed-step continuous, or adaptive state-space solves, and materializes named output TimeSeries.
- 2026-06-20: State-space vector semantic checks now reject role-incompatible members (`states` require state variables, `inputs` require input variables, and `outputs` require state/output variables), propagate mismatch status into dependent LinearOperator metadata, and cover the diagnostic through compiler and example-smoke tests.
- 2026-06-20: Source Newton and implicit-Euler DAE initial-option failures now surface solver-specific failure codes instead of leaking generic dynamic-component source codes. DAE state, derivative, and algebraic initial layout failures are distinguished in runtime tests, and the Newton diagnostic smoke expects `E-NEWTON-SOURCE-INITIAL-LAYOUT`.
- 2026-06-20: The source Newton `jacobian = source_linear_terms` hook now has a source-level runtime fixture and example-smoke coverage proving the provided-Jacobian method label, `source_linear_terms` step diagnostics, solved variables, residual vectors, and `.engres`/report-spec/report-HTML linear-step diagnostics.
- 2026-06-20: Component solver result-level tolerance, max-iteration, variable-scale, and dense-linear condition metadata are now aligned between runtime `.engres`, report-spec JSON, and their schemas, with the source-linear Newton smoke checking the fields.
- 2026-06-20: Component solver residual evaluations now carry source expression, source line, source reason, and dependency metadata through runtime `.engres`, report-spec solver-result residuals, and schemas, tightening W4/W9 residual traceability.
- 2026-06-20: Component solver report HTML largest-residual summaries now include source-line and dependency evidence, so narrow solver success/failure inspection does not require opening raw JSON for the first trace hop.

## Narrow/Not Claim Closure Matrix

This matrix turns the current `narrow`, `internal`, `planned`, `deferred`, and
`not production` solver statements into concrete closure gates. A row should be
removed from public limitation text only after its evidence gate is satisfied by
current tests and artifacts.

| Current narrow/not claim | Generic implementation work required | Evidence gate before claim can be removed | First aligned slices |
| --- | --- | --- | --- |
| General equation-system runtime is beyond the supported one/two-state source-equation shapes. | Lower arbitrary checked system equations into the shared solver IR, build named `x`, `u(t)`, `p`, `z`, output, and derivative layouts, and evaluate RHS/residual expressions through the typed expression tree. | Official multi-state non-thermal source example plus diagnostics for missing/duplicate derivatives, unsupported algebraic loops, unit mismatches, non-finite RHS, and generated RuntimeTimeSeries for every state/output. | Promote current two-state RHS/output evaluator into a shared system-IR adapter; add source spans and unit metadata to every lowered derivative and algebraic output equation. |
| Broad adaptive solving is not supported beyond source-equation, one-state thermal, internal continuous state-space, and the narrow dynamic component source path with selected affine/Newton algebraic materialization. | Run adaptive Heun over the shared dynamic IR, support component-coupled RHS evaluators, broad nonlinear algebraic coupling, interpolation and event hooks, and preserve accepted/rejected substeps per output sample. | Component-coupled adaptive fixtures expose fixed output-grid trajectories, substep diagnostics, event/failure artifacts, nonlinear algebraic limitation/failure artifacts, and report/IDE panels. | Reuse `TimeGrid`/adaptive step reports from source-equation, one-state thermal, state-space, and selected affine/Newton component source paths, then swap RHS source to shared dynamic IR. |
| State-space is limited to typed-block A/B fixed-step workflows. | Lower state-space operators into the shared IR, support operator algebra subsets, discrete adaptive policy, and composition with nonlinear/DAE/component residual paths. | State-space examples can be solved through the same dynamic/residual artifact path as source systems, with operator diagnostics and shared residual/RHS IDs in review/IDE. | Add adapter from `StateSpaceRhsEvaluator` matrices to shared solver IR variables/equations. |
| Component equations are limited to literal seeds, simple linear forms, and selected unit-parameterized coefficients. | Lower component-local arithmetic, derivative, behavior, nonlinear functions, affine units, compound units, and parameter values through one typed expression API. | Linear, dynamic, nonlinear, and DAE component examples all consume the same expression tree; unsupported unit algebra fails with source diagnostics instead of string-shape fallback. | Continue replacing one-off residual parsing with reusable expression AST metadata and evaluator entrypoints. |
| Broad args/object/non-arithmetic constructor bindings are not supported. | Introduce typed constructor-value IR for literals, imports, object fields, args values, and computed expressions with canonical unit conversion and provenance. | Dense linear, fixed-point, Newton, DAE, and dynamic component tests consume constructor values from the same parameter IR; report/review show provenance and rejected arity/type/unit cases. | Extend existing numeric/importable/pure-arithmetic parameter materialization into a shared `SolverParameter` adapter. |
| Fixed-point source solve is narrow and pivotable-linear only. | Detect general `x = g(x)` partitions from shared residual IR, add variable scaling, relaxation policy, residual history, and nonconvergence artifacts. | Success/failure fixtures cover coupled fixed-point loops, bad partitions, non-finite updates, tolerance/max-iteration options, and report/IDE residual history. | Move fixed-point update extraction from residual-shape helper to shared residual IR metadata. |
| Newton source solve is a narrow coupled component residual smoke. | Evaluate broad nonlinear residual trees from shared IR, add variable scales, Jacobian policy, line-search/trust-region diagnostics, finite-difference fallback, and source-line residual evidence. | Multi-variable unitful nonlinear source/component examples with nonlinear operators, singular/ill-conditioned/nonconvergent cases, and report/IDE Jacobian/residual diagnostics. | Use parsed residual ASTs for all Newton evaluations and surface inferred expression units in solver artifacts. |
| Implicit-Euler DAE is a narrow temperature example, not a broad DAE solver. | Infer robust state/algebraic/input/parameter partitions, initialize consistently with residual evidence, support configured mass matrices, event/failure artifacts, and keep unsupported methods explicit. | Coupled thermal/fluid DAE fixture plus inconsistent initial, mass-matrix, per-step Newton, and unsupported BDF diagnostics visible in artifacts. | Reuse shared residual expression ASTs for DAE residual evaluation and extend partition diagnostics. |
| Dynamic component solving is still narrow: explicit/semi-implicit fixed-step paths plus adaptive source solves with selected affine/Newton algebraic materialization only. | Build dynamic component layouts from shared IR, support algebraic-free and semi-implicit paths with parameterized derivatives, selected nonlinear algebraic solves, adaptive stepping, and selected algebraic outputs. | Dynamic component fixtures cover multi-state, parameters, TimeSeries inputs, algebraic trajectories, adaptive substeps, nonconvergence, unsupported nonlinear adaptive algebraic graphs, and report/IDE timestep diagnostics. | Move dynamic residual graph construction fully onto typed expression lowering and expose inferred units in artifacts. |
| Behavior graph solving is limited to identity-wrapper explicit-Euler RHS smokes plus narrow Predictor/external identity-wrapper DAE residual samples. | Lower behavior calls into first-class solver IR nodes with deterministic replay, model loading contracts, process policy, range warnings, finite-difference policy, and generic dynamic/DAE coupling. | Delay, Predictor, and external examples affect dynamic and DAE solves, safe/repro failures are enforced, and behavior node warnings/failures appear in report/IDE. | Connect behavior-node contracts to shared expression/RHS metadata instead of component-local string scans; add a replay-safe delay history policy before enabling delay in DAE Newton samples. |
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
- Keep variable scale policy, source residual scale overrides, and largest-normalized-residual summaries wired through runtime/report artifacts; broaden this beyond the current source Newton/DAE residual paths.
- Route supplied analytic/source-linear Jacobian, finite-difference Jacobian, and future JIT Jacobian through one policy; finite-difference/source-linear policy labels and dense linear Newton-step diagnostics are now carried through solver/runtime/report artifacts, while future JIT Jacobian selection remains planned.
- Keep line-search controls and diagnostics wired through solver/runtime/report artifacts; keep singular/ill-conditioned dense-linear pivot diagnostics visible in solver/runtime/report artifacts; add trust-region controls and broader nonconvergence artifacts.
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
- Emit RuntimeTimeSeries for every named state and selected algebraic output. Source-equation and typed-block state-space scalar outputs are implemented; selected explicit and semi-implicit dynamic-component algebraic output fixtures are implemented; broad shared-IR/component-coupled outputs remain.

Evidence gate:

- Official/integration examples for multi-state component dynamics with TimeSeries inputs.
- Adaptive diagnostics include accepted/rejected substeps, event/failure metadata, and fixed output-grid trajectories.

### W6. DAE Completion

Goal: make DAE support a real solver path rather than a narrow implicit-Euler
smoke.

Tasks:

- Infer state/algebraic/parameter/input partition from shared IR.
- Implement consistent initialization with residual evidence and diagnostics.
- Extend the current dimensionless source `mass_matrix` option into a unit-aware shared-IR mass-matrix policy.
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
- Source implicit-Euler DAE solve requests now accept `mass_matrix = identity`, scalar broadcast, diagonal vectors, or dense square dimensionless matrices; configured mass derivatives feed source `der(...)` residual symbols, final residual evaluation, report artifacts, and layout/value diagnostics.
- Source implicit-Euler DAE solve requests now materialize scalar and fixed-step TimeSeries component inputs into DAE residual evaluation, including algebraic initialization, implicit-step Newton samples, final residual samples, trajectory artifacts, and example-smoke coverage through `tests/runtime/dae_timeseries_input_from_source.eng`.
- Source-equation ODE simulations now route `solver = adaptive_heun` through the shared `SourceRhsEvaluator` path for generic numeric state layouts, preserving fixed output-grid trajectories and adaptive substep diagnostics.
- Source ODE RHS evaluation now pre-parses derivative coefficients and RHS expressions through the shared arithmetic expression parser, preserving unit-literal metadata and removing per-sample RHS string parsing for fixed-step and adaptive source simulations.
- Source ODE RHS parser construction now receives typed input and parameter symbol metadata from runtime layouts, so source derivative expressions preserve state/input/parameter units instead of name-only RHS symbols.
- Source ODE RHS numeric literals with known built-in units now convert to canonical solver values during shared expression parsing while preserving unknown compound suffix compatibility.
- State-space vector declarations now validate member roles before operator compatibility checks, preventing role-incompatible state/input/output vectors from being treated as shape-compatible solver inputs.
- Newton solver results now retain accepted line-search scale and trial-count diagnostics per iteration, and runtime/report component solver step diagnostics expose that metadata for source Newton and DAE implicit-step Newton solves.
- Dense linear solves now distinguish exact zero-pivot singular failures from tolerance-level ill-conditioned pivot failures, retain min/max pivot magnitudes plus a pivot-condition estimate on successful solves, and expose those diagnostics through linear residual graph and component solver report artifacts.
- Newton results now retain the Jacobian policy label and per-iteration dense-linear step diagnostics, and source Newton/implicit-Euler DAE component step diagnostics expose those fields in report-spec artifacts.
- Source Newton provided-Jacobian coverage now includes `tests/runtime/newton_source_linear_jacobian.eng`, which solves a linear source residual graph with `jacobian = source_linear_terms` and verifies method/policy artifacts in `.engres`, report-spec JSON, and report HTML through `eng test examples`.
- Component solver result-level metadata now records tolerance, max iterations, variable-scale policy/range, and dense-linear pivot condition fields in runtime `.engres` with matching result/report-spec schema entries.
- Component solver residual evaluation artifacts now include source expression, source line, source reason, and dependency lists in runtime `.engres` and report-spec solver-result residual entries, so residual values can be traced without relying only on assembly-summary metadata.
- Component solver step diagnostics now include source expression, source line, and source reason for the largest named residual when that residual exists in solver metadata, so per-step Newton/fixed-point/DAE evidence is traceable without manually joining separate artifact arrays.
- Component solver report HTML now includes source-line and dependency summaries for largest residuals.
- Shared arithmetic expression trees now support dimensionless one-argument `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`, and `atan` calls, including source Newton, source RHS, and source DAE fixture coverage plus parser diagnostics for unitful function arguments.
- Component equation semantic dimension checks now reject unitful arguments to the supported dimensionless math-function subset before runtime residual solving.
- Source-equation RK4 RHS simulation now evaluates dimensionless `sin`/`cos` calls through the shared arithmetic expression tree, including algebraic output materialization and plot/report artifact checks.
- Source implicit-Euler DAE residual solves now evaluate dimensionless `sin`/`cos` calls through the shared arithmetic expression tree, including per-step diagnostics and report artifact checks.
- Newton residual vector history now feeds source Newton and implicit-Euler DAE component step diagnostics with largest residual index/name/value evidence in report-spec artifacts.
- Static component residual graph construction now preserves unsupported linearization status, failure code, and failure reason instead of silently dropping unsupported component-equation terms; dense linear, evaluator, runtime, and report artifacts retain those residuals as `unsupported_linearization`.
- Newton variable scaling is now part of `NewtonOptions`, finite-difference perturbation, scaled dense Newton-step solves, source Newton bridge metadata, source DAE implicit-step Newton options, runtime `.engres`, and report-spec artifacts; source bridges derive default scales from assembly unknown quantity/unit metadata.
- Source Newton and implicit-Euler DAE step diagnostics now expose full raw and normalized residual vectors in runtime `.engres` and report-spec artifacts, so narrow nonlinear/DAE smokes retain per-iteration residual evidence instead of only the largest residual summary.
- Fixed-point solver results now retain per-iteration update residual vectors, and source fixed-point component artifacts expose raw/normalized variable-update vectors, variable-scale policy, and largest update identity in runtime `.engres` and report-spec step diagnostics.
- Source fixed-point solve requests now accept scalar or per-unknown vector `initial` values with unit conversion, and vector layout mismatches produce fixed-point-specific failure artifacts instead of silently broadcasting a scalar-only guess.
- Source Newton and implicit-Euler DAE solve requests now remap shared initial-option parser failures into solver-specific failure artifacts, including distinct DAE state, derivative, and algebraic layout codes.
- Dynamic component algebraic step diagnostics now preserve raw and normalized residual vectors plus largest residual identity for fixed-point and semi-implicit linear algebraic substeps in runtime `.engres` and report-spec artifacts.
- Dynamic component residual lowering now folds materialized component parameters into linear derivative coefficients, so source examples such as `C * der(port.T) eq port.Q` solve through the residual-graph semi-implicit path instead of requiring literal derivative coefficients.
- Dynamic component semi-implicit source solves can now use parsed derivative residual RHS expressions while retaining dense linear algebraic residual graph solves, with runtime smoke coverage for a dimensionless `sin(state)` RHS fixture.
- Dynamic component semi-implicit source solves can now fall back to per-step Newton algebraic residual solves for parsed dimensionless nonlinear algebraic residuals, with runtime smoke coverage for a nonlinear boundary residual fixture.

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
