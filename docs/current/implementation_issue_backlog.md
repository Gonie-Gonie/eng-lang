# Implementation Issue Backlog

This backlog captures post-1.0 implementation work that is outside the current
stable-core claim. Move these entries into GitHub Issues when issue-write
permissions are available.

## Formatter

Title: `formatter: add an EngLang source formatter for official examples`

Definition of Done:

- Add an `eng fmt` command or equivalent formatter entrypoint.
- Preserve comments and stable source semantics.
- Format `args`, `schema`, `system`, `report`, `where`, and `with` blocks
  consistently.
- Keep official examples formatter-clean.
- Add regression tests for formatter output.
- Document the formatter workflow in development docs.

Current coverage:

- `eng fmt <file.eng>` exposes the compiler formatter with stdout, `--check`,
  and `--write` modes.
- The formatter is source-preserving: it normalizes block indentation and
  trailing whitespace while preserving comments and string contents.
- Regression tests cover `args`, `schema`, `system`, `report`, `where`, and
  `with` block indentation, comment preservation, string brace handling,
  idempotence, and semantic-summary stability for valid source.
- `eng test examples` checks all files under `examples/official` for
  formatter-clean source.
- The development workflow documents how to run formatter write/check modes.

## Runtime Optimization / JIT

Title: `jit: connect kernel IR to runtime optimization without native claims`

Current coverage:

- `eng_jit` records kernel-plan candidates, backend selection metadata, and
  per-candidate interpreter executor/fallback reasons.
- Runtime optimization has `eng-kernel-ir-v1` plus an interpreter executor for
  element-wise TimeSeries arithmetic, TimeSeries statistics reductions,
  trapezoid integration, scalar residual, finite-difference Jacobian, and
  Newton-step and explicit-Euler solver-step correctness tests.
- Checked TimeSeries arithmetic, `summarize ... by [...]`, and
  `integrate(... over Time)` metadata can lower to executable interpreter
  `KernelIr` when runtime values/timestep are supplied.
- Component assembly residual graphs can lower checked assembly equations into
  scalar residual evaluator `KernelIr`; the official Thermal assembly fixture
  executes that IR and finite-difference Jacobian path in tests.
- Square component assembly residual graphs are also surfaced as
  `component_residual_jacobian` kernel-plan candidates, with interpreter
  support backed by finite-difference evaluation over the scalar residual IR.
- Square component assembly residual/Jacobian paths are surfaced as
  `component_newton_step` candidates for a single dense Newton update; nonlinear
  iteration remains outside this kernel candidate.
- Continuous state-space A/B operators can lower checked `der(x) eq A * x + B *
  u` metadata into an executable scalar RHS `KernelIr`, and that RHS can execute
  one interpreter explicit-Euler solver-step kernel; full simulation still
  remains on the normal runtime solver path.
- CLI example smoke checks kernel candidates, interpreter executor fallback
  metadata, component residual/Jacobian/Newton-step kernel candidates,
  executable CSV/statistics interpreter-kernel samples, CSV heat-rate,
  multi-statistics, component-graph solver, state-space RHS, and state-space
  solver-step benchmark target coverage, and native-backend non-availability
  without making a speedup claim.
- `report_spec.json`, `report.html`, and the IDE Kernel panel surface the
  selected backend, kernel candidates, executor status, and fallback reason as
  inspection metadata without presenting acceleration.
- `eng.exe jit-bench` remains a normal-runtime timing harness and makes no
  speedup claim; its `benchmark_targets` field records which checklist target
  families were observed in the current source's kernel plan, and its
  `kernel_executor_samples` field records deterministic interpreter-kernel
  sample executions for lowerable candidates.
- `dev.bat jit-check` asserts benchmark smoke coverage for the CSV heat-rate,
  multi-statistics fusion, residual evaluation, Thermal component assembly
  Newton-step, and continuous state-space RHS/solver-step target families.

Definition of Done:

- Lower real TimeSeries arithmetic/integration candidates from checked source
  into executable interpreter IR using runtime inputs.
- Lower residual evaluator, state-space RHS, and Jacobian evaluator kernels from
  checked source and solver assembly artifacts, not just hand-built IR tests.
- Keep native backend selection behind `not_available` until native execution
  exists and is benchmarked.
- Report selected kernel/fallback reason in report/IDE surfaces without
  presenting it as acceleration.

## IDE Inspectors

Title: `ide: implement variable/unit/schema/TimeSeries inspectors`

Definition of Done:

- Variable table shows name, type, quantity, display unit, canonical unit,
  source expression, and source line.
- Unit table shows display/canonical conversion metadata.
- Schema table shows columns, constraints, missing policies, source hashes, and
  parse/conversion failures.
- TimeSeries inspector shows start/end, timestep or sample spacing, row count,
  missing count, source column, quantity/unit, canonical/display unit, and axis.
- Solver inspector shows system name, state/algebraic/input/parameter/output
  lists, timestep, method, tolerance, iteration count, convergence status, and
  failure reason from runtime solver artifacts.
- TimeSeries result panel shows solver state trajectories plus input/output
  series metadata and point counts without requiring raw JSON inspection.
- Measured-vs-simulated workflow shows `weather_data`, `measured_data`,
  `sim.T_zone`, `rmse_T`, validation, time alignment, and two-series plot data.
- Add automated IDE smoke coverage.

Current coverage:

- IDE smoke covers schema/TimeSeries/metric/validation/time-alignment metadata
  for measured-vs-simulated and schema parse/conversion failure counts for a
  data-quality fixture.
- IDE panels expose state-space solver result rows with state trajectories,
  input/output series metadata, timestep, tolerance, iteration count,
  convergence status, and failure reason from `report_spec.json`/`result.engres`.
- IDE TimeSeries inspector rows expose component solver state/algebraic
  trajectory metadata plus component solver failure code/reason when a failed
  internal dynamic-component result still carries trajectory points.
- IDE panels expose system equation dependency rows from `system_ir` so variable
  dependencies and derivative states are visible without raw JSON.
- IDE smoke covers residual dependency rows from
  `assembly_summary[].residual_graph.dependencies` and behavior graph nodes for
  delay, Predictor, and external adapter seeds from `component_graph`.
- IDE smoke covers the solved Thermal component assembly path by checking
  boundary RHS equations, dense linear solve status, solved variables, and
  normalized residual metadata from `assembly_summary`.
- IDE smoke covers the Kernel panel by checking the official CSV workflow's
  `timeseries_integrate` candidate and interpreter fallback reason from
  `report_spec.json`.
- `artifacts-check` snapshots the official CSV workflow's promoted data source
  hash, TimeSeries axis metadata, and HeatRate-to-Energy integration unit
  contract across `review.json`, `report_spec.json`, and `result.engres`.
- `artifacts-check` snapshots the measured-vs-simulated repro-profile
  workflow's RMSE `TemperatureDelta` metric, validation result, matched
  measured/simulated time alignment, SolverResult state trajectory, and
  two-series PlotSpec contract.
- `artifacts-check` snapshots the official one-state solver artifact contract
  across `review.json` `simulation_results[].solver_results`,
  `report_spec.json` `system_ir[].solver_results`, and `result.engres`
  `solver_result` fields.
- `eng test examples` directly asserts the measured-vs-simulated SolverResult
  state/input/parameter/output lists across review, result, report spec, and
  report HTML artifacts.
- `artifacts-check` snapshots the official multi-state, Thermal component
  assembly, and constrained multi-domain boundary solve examples so solver
  trajectories, residual graph solve status, solved variables, and residual
  counts are covered by golden baselines.
- `artifacts-check` snapshots the internal behavior-node fixture so delay,
  Predictor, and external adapter artifact statuses remain explicit until
  language-level behavior graph solving is wired.
- `StateTrajectory::time_value_points` keeps SolverOutput state trajectories
  directly convertible to TimeSeries-style `(time, value)` points, and runtime
  system/component artifact adapters use that helper.
- Fixed-step method dispatch now lives behind the solver module as
  `FixedStepMethod`/`solve_fixed_step_ode`, so runtime materialization calls the
  solver API instead of carrying a local dispatch wrapper.
- `eng test examples` directly exercises the fixed-step ODE solver API for
  two-state explicit Euler/RK4 trajectories, final partial timestep handling,
  and non-finite RHS/update failure artifacts.
- `TimeGrid::step_dt_s` drives fixed-step ODE and dynamic-component state
  updates, so non-divisible durations use a shorter final integration step
  rather than overshooting the requested duration.
- Explicit Euler RHS evaluation samples at the start of each fixed-step
  interval, which keeps time-dependent inputs aligned with the state being
  advanced.
- `SolverInput::validate_layouts` rejects non-finite initial state, input, and
  parameter values before solver algorithms run.
- State-space RHS/discrete seeds reject non-finite A/B matrix, sampled input,
  derivative, and updated state values before those values can enter
  trajectories.
- Dense linear solver seeds reject non-finite matrix/RHS values and invalid
  tolerances before pivoting.
- Newton solver seeds reject non-finite initial guesses before residual or
  Jacobian evaluation.
- Solver residual diagnostics use a shared scaled Euclidean norm helper to
  avoid overflow for large finite residual values.
- Fixed-point, Newton, and dense linear solver seeds reject non-finite
  intermediate values produced by relaxation, line search, finite differences,
  or elimination.
- ResidualGraph linear-system assembly rejects non-finite residual
  coefficients or RHS values before dense solving.
- ResidualEvaluator rejects non-finite residual inputs, scales, evaluated
  residuals, normalized residuals, and residual norms instead of emitting
  non-finite diagnostics.
- DAE seeds reject non-finite state-derivative intermediates, mass-matrix
  application results, and algebraic-initialization inputs.
- Fixed-step ODE, fixed-point, and dynamic-component solver seeds reject
  non-finite RHS/update values before those values can enter trajectories or
  algebraic artifacts.
- `SolverInput::validate_layouts` now validates non-empty `OutputLayout`
  entries against declared outputs and state quantity/unit metadata while still
  allowing empty output layouts for internal vector-output seeds.
- The supported one-state thermal runtime path now delegates RHS evaluation and
  fixed-step execution to `solver::thermal::solve_first_order_thermal`; runtime
  materialization only recognizes the system shape and prepares canonical input.
- `RuntimeSystemSolution::from_solver_result` and `from_solver_trajectory`
  provide the explicit SolverResult-to-runtime artifact adapter used by
  one-state thermal and state-space materialization paths.
- `RuntimeComponentSolution::from_solver_assembly` and
  `from_dynamic_solver_result` provide explicit component assembly/SolverResult
  adapters for residual-graph and dynamic-component artifact materialization.
- `RuntimeSystemSolution::to_report_solution` and
  `RuntimeComponentSolution::to_report_solver_result` keep the
  runtime-artifact-to-report/review projection explicit, including dynamic
  component per-step nonconvergence failure artifacts.

Title: `ide: add side-effect artifact panels`

Current coverage:

- IDE `Effects` tab shows output-manifest artifacts, run-log messages,
  process results, and test results from the latest run.
- IDE smoke covers output manifest, run log, process results, and test results.

Definition of Done:

- Output manifest viewer lists generated artifacts and side-effect records.
- Run log viewer shows `print` and `log` records with level/source line.
- Process result viewer shows command, args, cwd, status, stdout/stderr, and
  duration.
- Test result viewer shows named tests, assertions, golden checks, and failures.
- Safe/normal/repro profile diagnostics are visible.
- Missing artifact files do not crash the IDE.

## Dynamic System I/O

Title: `system: support explicit TimeSeries input declarations`

Definition of Done:

- `input T_out: TimeSeries[Time] of AbsoluteTemperature [degC]` parses and
  checks.
- `simulate ... with { T_out = weather_data.T_out }` validates axis and
  quantity against the explicit input contract.
- Missing input, wrong type, wrong quantity, wrong axis, missing/invalid
  timestep, missing/unsupported solver, and unknown system diagnostics are
  covered by compiler tests and `examples/05_error_messages` smoke fixtures.
- `sim.T_zone` remains materialized as a typed TimeSeries in result/report/IDE
  artifacts.
- Docs distinguish the current scalar input plus TimeSeries binding rule from
  the explicit TimeSeries input form.

Current coverage:

- Parser type splitting preserves explicit dynamic input contracts such as
  `TimeSeries[Time] of AbsoluteTemperature [degC]`; compiler regression tests
  cover the parsed quantity kind, display unit, and canonical unit.
- Official measured-vs-simulated and multi-state thermal examples declare
  `T_out` as `TimeSeries[Time] of AbsoluteTemperature [degC]` and bind it with
  `simulate ... with { T_out = weather_data.T_out }`.
- Simulate input validation resolves promoted CSV column types and checks the
  explicit input contract's axis and quantity before runtime execution.
- Simulate input/timestep/solver diagnostics use the checklist canonical codes:
  `E-SIM-MISSING-INPUT`, `E-SIM-INPUT-QTY-MISMATCH`,
  `E-SIM-INPUT-AXIS-MISMATCH`, `E-SIM-TIMESTEP-INVALID`, and
  `E-SIM-SOLVER-UNSUPPORTED`.
- Error-message fixtures cover missing TimeSeries input, non-TimeSeries input,
  wrong TimeSeries quantity, wrong TimeSeries axis, missing/invalid timestep,
  missing/unsupported solver, and unknown system cases.
- Runtime materializes `sim.T_zone` from `SolverResult` as a typed
  TimeSeries; runtime tests assert the explicit `T_out` contract, SolverResult
  status, state trajectory, RMSE alignment reference, and `sim.T_zone`
  registration.
- `docs/guide/language_grammar.md` distinguishes the explicit TimeSeries input
  contract from the earlier scalar input plus TimeSeries-binding compatibility
  path.
- System solver artifacts carry nullable `failure_code`; unsupported simulated
  shapes surface `E-SIM-SYSTEM-SHAPE-UNSUPPORTED` alongside the failure reason
  in result/review/report data.

## State-Space

Title: `state-space: close stable workflow boundaries after vector runtime seed`

Definition of Done:

- Keep the vector runtime seed clearly scoped until the stable workflow
  boundary is decided.
- Current runtime covers `StateVector`, `InputVector`, and `LinearOperator`
  metadata, operator row/column checks, non-rectangular matrix diagnostics,
  non-numeric/non-finite matrix-entry diagnostics, unsupported unitful
  matrix-entry diagnostics, inverse-time coefficient checks and per-second
  canonicalization for derivative-compatible source units,
  report/review/IDE canonical operator matrix and named-entry summaries,
  continuous and discrete A/B execution, multi-state fixed-step Euler/RK4
  trajectories, state trajectory TimeSeries, `OutputLayout` preservation across
  solver input/result contracts, and solver-inspector metadata for states,
  inputs, outputs, timestep, tolerance, iterations, convergence, and failure
  reason. Plot/report output, IDE inspector support, and continuous/discrete
  state-space smoke fixtures are in place for the current seed scope.
- Discrete and continuous state-space fixed-step execution now live in
  `solver::state_space`; runtime materialization supplies the TimeSeries/scalar
  input sampler and adapts the `SolverResult`.
- State-space runtime materialization consumes compiler-provided
  `LinearOperatorInfo::canonical_matrix` instead of reparsing operator
  expression strings.
- Remaining supported-workflow work includes broader operator algebra and
  coefficient-unit policy, and public stability wording.
- No nonlinear/DAE/adaptive or component-coupled solver claim is made.

## Nonlinear / DAE Solver

Title: `solver: integrate nonlinear and DAE paths beyond standalone algorithm seeds`

Current coverage:

- Runtime has a standalone damped Newton algorithm seed with finite-difference
  fallback, supplied analytic/JIT Jacobian hook, largest-residual summary,
  residual-history, convergence-status, invalid-option, and nonconvergence
  tests exposed through the solver API. Singular Newton linear solves and failed
  line-search candidates are returned as `NewtonResult` failure artifacts so
  callers can preserve solver diagnostics instead of losing the iteration state.
- Runtime also has dense linear and solver-API fixed-point algorithm seeds; the
  CLI example smoke now covers linear residual graph convergence and singular
  failure artifacts plus fixed-point convergence and nonconvergence failure
  artifacts.
- `eng test examples` now directly exercises the solver-API Newton and
  implicit-Euler DAE seeds, including two-variable nonlinear convergence,
  supplied Jacobian hook use, Newton nonconvergence failure artifacts,
  state/algebraic DAE convergence, mass-matrix derivative use, inconsistent
  initial-condition failure, and per-step DAE nonconvergence artifacts.
- Runtime has a standalone implicit-Euler DAE seed over
  `F(x, xdot, z, u, t, p)` with optional mass matrix, initial-condition
  consistency checks, algebraic-variable initialization, ODE residual,
  algebraic-variable, mass-matrix, inconsistent-initial-condition, and step
  nonconvergence tests exposed through the solver API.
- Runtime has a standalone dynamic-component explicit-Euler seed that supports
  algebraic-free two-state updates and semi-implicit fixed-point algebraic
  solves at each timestep, updates state trajectories, carries algebraic
  trajectories through the common `SolverResult` output contract, and returns
  timestep-level convergence/failure diagnostics. Component solver result
  artifacts can now carry those state/algebraic trajectories and timestep
  diagnostics through report spec, HTML, `.engres`, and the IDE assembly
  summary.
- Dynamic-component RHS evaluation can be driven from derivative-form
  `ResidualGraph` equations, giving the internal explicit-Euler seed a concrete
  residual-to-RHS bridge for algebraic-free dynamic systems.
- Runtime exposes a residual-graph explicit-Euler dynamic-component entrypoint
  that validates solver input layouts, rejects algebraic variables for the
  algebraic-free path, reuses the common dynamic-component fixed-step loop, and
  returns `SolverResult` state trajectories plus timestep diagnostics.
- Runtime also exposes a residual-graph semi-implicit dynamic-component
  entrypoint for derivative residuals plus linear algebraic residuals; it solves
  algebraic variables with the dense linear solver at each timestep, reuses the
  same fixed-step state update loop, and reports algebraic solve failures through
  `SolverResult` diagnostics and step diagnostics.

Definition of Done:

- Wire Newton or quasi-Newton solving into language-level nonlinear residual
  systems.
- Wire implicit Euler DAE solving into language-level examples and artifacts.
- Add runtime examples for a small nonlinear system and a small implicit DAE.
- Add BDF policy after implicit Euler integration is stable.
- Keep unsupported paths explicit in review/report/IDE artifacts until
  integrated solving is truly available.

## Delay / Predictor / External Behavior

Title: `solver: integrate behavior graph nodes into numeric evaluation`

Current coverage:

- Runtime has a solver-API delay buffer seed with linear and previous-sample
  interpolation policies, explicit initial-history policy, relationship
  artifacts, out-of-order/history-underflow diagnostics, and solver-style
  behavior-node evaluation tests.
- `DelayBehaviorNode::evaluate_rhs` can feed delayed values into a fixed-step
  RHS closure and rejects non-finite delayed RHS derivatives.
- `eng test examples` now directly exercises solver-API behavior node numeric
  evaluation: delay interpolation and history-underflow failure artifacts,
  Predictor valid-range warnings and output-layout failures, and external
  behavior deterministic repro execution, safe-profile rejection, and adapter
  failure wrapping.
- Component local expressions now diagnose invalid `delay(signal, duration)`
  calls, including missing arguments, unknown component signals, and
  non-duration delay values. Signals can be declared component `port.variable`
  references or prior component-local expressions with resolved quantity/unit
  metadata; nested delay behavior expressions also preserve their delayed
  signal quantity/unit contract when used as behavior-call inputs.
  Example-smoke fixtures cover each public component behavior diagnostic code.
- Component local Predictor and external behavior expressions now diagnose
  invalid seed syntax and unknown component signals before they become
  behavior-node metadata.
- Runtime has a solver-API Predictor behavior contract wrapper with input/output
  quantity-unit metadata, valid-range warnings, provenance/model hash,
  differentiability flag, solver Jacobian policy, and evaluation/failure tests.
- `PredictorBehaviorNode::evaluate_rhs` can feed predictor outputs into a
  fixed-step RHS closure and rejects non-finite predictor-driven derivatives.
- Runtime has a solver-API external function/process behavior wrapper with typed
  input/output contracts, provenance hash, determinism metadata, safe/repro
  profile policy, range warnings, adapter failure propagation, and tests.
- `ExternalBehaviorNode::evaluate_rhs` can feed external behavior outputs into a
  fixed-step RHS closure while preserving profile checks and adapter failures.
- Component artifacts distinguish delay calls as
  `delay_call_runtime_buffer_seed_not_integrated` and Predictor calls as
  `predictor_call_contract_seed_not_integrated`; external behavior expressions
  report `external_behavior_wrapper_seed_not_integrated`.
- Review/report/component-graph artifacts and the IDE Assembly panel expose
  behavior nodes with source navigation metadata, inferred input/output
  quantity-unit contract metadata, and diagnostic channels for delay
  history-underflow, Predictor valid-range warnings, and external
  adapter/profile failures.
- Runtime component solver artifacts now add an explicit reason/failure note
  when behavior graph nodes are present but not yet integrated into numeric
  residual evaluation.
- `examples/internal/25_component_behavior_nodes` exercises valid delay,
  Predictor, and external adapter behavior-node artifacts, including prior
  component-local signal contract resolution, in the CLI example smoke path.

Definition of Done:

- Expand `delay(x, tau)` type checking beyond component port variables into
  full behavior graph expressions.
- Wire delay behavior nodes into RHS/residual evaluation for supported solver
  paths.
- Wire Predictor behavior nodes into supported solver paths and report/IDE
  artifacts.
- Wire external behavior wrappers into supported solver paths and report/IDE
  artifacts.
- Report and IDE should show full behavior contracts and invalid/extrapolated
  behavior warnings once behavior nodes are connected to runtime evaluation.

## Class/Domain Objects

Title: `class: close runtime object support before any stable claim`

Definition of Done:

- Runtime object representation exists for class literals and nested objects.
- Field access produces checked runtime values.
- Default fields, validation results, zero-argument metadata methods,
  copy-with behavior, and IDE object-summary inspection are covered by tests.
- Report/review artifacts include object summaries and validation results.
- IDE completion/hover shows fields, defaults, required fields, and units.
- Docs keep classes separate from systems/components and avoid solver claims.

Current coverage:

- `examples/official/19_class_object` exercises typed class fields/defaults,
  object literals, nested object references, field access metadata, validation
  blocks, zero-argument metadata methods, immutable copy-with, and class/object
  report sections.
- Compiler semantic analysis records `class_summary` and `object_summary`,
  evaluates supported object validation rules, checks required/unknown/typed
  fields, and validates zero-argument method declarations/calls.
- CLI smoke checks class/object review and report artifacts including
  validation count, method count, copy-with construction, field access, and
  generated report HTML.
- Error-message fixtures cover missing required fields, unknown fields,
  incompatible field values, validation failures, method return mismatches,
  unknown method calls, and copy-with unknown sources.
- IDE object-summary inspection and LSP hover/member/object-literal completion
  expose class fields, defaults, required fields, and units.
- `docs/guide/class_object.md`, the feature maturity matrix, and stable-core
  scope keep classes separate from systems/components and state that runtime
  object dispatch/lowering, method arguments, mutation, inheritance, and
  simulation lowering remain deferred.

## Component Graph

Title: `component: implement graph inspector without numeric solver claims`

Definition of Done:

- Component instances, ports, connect edges, domain labels, type arguments,
  medium/frame/axis metadata, and source spans are exposed as a graph artifact.
- Duplicate connection, unknown port, unconnected port, invalid endpoint,
  incompatible domain, incompatible medium/frame/axis, and unsupported
  connect-pattern diagnostics are covered.
- Component boundary equation RHS/type diagnostics reject non-numeric RHS values
  and units that are incompatible with the connected port signal quantity, and
  reject unknown `port.variable` signal paths before residual assembly.
- IDE graph panel can navigate connections back to source.
- Report summarizes connection graph and limitations.
- Production component graph solving remains Planned.

Current coverage:

- IDE Assembly panel renders component graph components, ports, connections,
  labels, statuses, and source-line navigation from `component_graph`.
- IDE Assembly panel renders behavior nodes plus generated equation and
  evaluated residual rows from assembly artifacts.
- Internal boundary fixtures can promote component-local `port.signal = literal`
  expressions into boundary residual equations and solve square dense linear
  residual graphs with explicit variable, residual, and RHS artifacts.
- Runtime square residual graph solves are routed through
  `solve_linear_residual_graph`, which owns linear system assembly, dense solve,
  residual-norm evaluation, named variable output, and singular/ill-conditioned
  failure propagation.
- `examples/official/21_thermal_component_assembly` exercises the focused
  Thermal assembly path with generated connection equations, component-boundary
  RHS equations, a square residual graph, and dense linear solver artifacts.
- `examples/official/22_multi_domain_boundary_solve` exercises a constrained
  Thermal/Fluid/MechanicalNode boundary path with generated connection
  equations, component-boundary RHS equations, a square residual graph, and
  dense linear solver artifacts while preserving the non-production
  multi-domain limitation.
- `examples/internal/23_component_boundary_singular` exercises the same square
  residual graph path when the dense matrix is singular and requires a
  `linear_solve_failed`/`E-LINEAR-SINGULAR` artifact.
- `examples/internal/24_component_boundary_overdetermined` exercises the
  overdetermined residual graph limitation path and requires a
  `not_solved_overdetermined`/`E-ASSEMBLY-OVERDETERMINED` artifact instead
  of attempting a dense solve.
- Checklist 9.4 test names are covered by current repo test surfaces as
  follows: `linear_algebraic_thermal_node.eng` maps to
  `examples/official/21_thermal_component_assembly` and
  `examples/internal/22_component_boundary_solve` smoke checks;
  `linear_singular_system.eng` maps to
  `examples/internal/23_component_boundary_singular`; fixed-point
  `small_loop` and `nonconvergence` cases are runtime algorithm tests in
  `crates/eng_runtime/src/solver/algorithms/fixed_point.rs` until a
  language-level algebraic-loop fixture surface is added.
- Runtime has an internal dynamic-component solver seed, but component graph
  assembly is not yet lowered into that numeric path.
- Component connection/assembly diagnostics use checklist canonical codes for
  domain mismatch, medium mismatch, unknown port, unconnected port,
  underdetermined/overdetermined assembly, and algebraic-loop warnings.
- `EquationAssembly::dynamic_component_split` validates state/algebraic/input/
  parameter role splits, rejects duplicate or inconsistent variable
  classifications, and produces solver layouts for the dynamic-component seed
  boundary.

Title: `assembly: harden generated equations and residual graph artifacts`

Definition of Done:

- Collect component instances, ports, connection sets, and generated connection
  equations.
- Preserve component-local boundary equation seeds with RHS literals for
  internal square algebraic fixtures.
- Record state/algebraic/input/output classification, equation count, unknown
  count, residual list, dependency graph, algebraic-loop seed, sparsity
  placeholder, solver-plan placeholder, and runtime parameter references with
  stable indices separate from solved variables.
- Report and IDE show generated equations, source-line links, generated
  reasons, residual values, normalized residuals, scale policy, and residual
  graph metadata.
- Runtime residual evaluation accepts solver-provided tolerance and
  per-residual scale overrides; current component artifacts still default to
  unit/quantity scale policy until a user-facing scale surface is added.
- Runtime residual evaluation consumes structured `x`, optional `xdot`, `z`,
  `u`, `p`, and `t` inputs, returns raw and named normalized residuals, and is
  repeatable without report-layer dependencies.
- Runtime linear algebraic solves consume `ResidualGraph` through the solver
  API instead of inlining matrix solve plumbing in component artifact code.
- Runtime `component_solutions` and report `solver_result` expose
  nullable `failure_code`/`failure_reason` aliases plus `largest_residuals`,
  capped to the top normalized residuals for direct report/IDE inspection.
- IDE Assembly panel shows residual dependency rows from
  `assembly_summary[].residual_graph.dependencies` and surfaces solver failure
  code/message plus `largest_residuals` aliases when present.
- Under/overdetermined cases produce diagnostics or limitation artifacts.
- No production multi-domain solver claim is made.

Current coverage:

- Compiler semantic assembly records component instances, ports, connection
  sets, generated connection equations, boundary RHS equation seeds, equation
  and unknown counts, residual lists, dependency rows, algebraic-loop seeds,
  jacobian-sparsity placeholders, solver-plan placeholders, and domain plans.
- `assembly_summary` and `component_graph` review/report artifacts preserve
  generated reasons, source-line navigation data, residual graph status,
  dependency rows, scale policy, and limitation metadata for report and IDE
  consumers.
- Runtime builds solver residual graphs from compiler assembly artifacts and
  evaluates raw and normalized residuals through `solver::residual` without
  depending on report-layer JSON.
- `ResidualEvaluationInput` accepts solver-provided tolerance and optional
  per-residual scale overrides; tests cover default unit/quantity scales,
  user-provided scale overrides, invalid scales, structured `x`/`xdot`/`z`/`u`
  inputs, and repeatable named residual evaluation.
- Square algebraic assembly paths call `solve_linear_residual_graph`, and
  runtime component artifacts adapt the solver result instead of inlining dense
  matrix solve plumbing in report code.
- Runtime `component_solutions`, report `solver_result`, and IDE Assembly
  smoke expose nullable `failure_code`/`failure_reason` aliases plus capped
  `largest_residuals` for solved, singular, underdetermined, and overdetermined
  paths.
- Official/internal fixtures cover the focused Thermal square solve, constrained
  multi-domain boundary solve, singular dense solve failure, and overdetermined
  limitation artifact while keeping production multi-domain solving out of the
  public claim.
