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

## Runtime Optimization / JIT

Title: `jit: connect kernel IR to runtime optimization without native claims`

Current coverage:

- `eng_jit` records kernel-plan candidates, backend selection metadata, and
  per-candidate interpreter executor/fallback reasons.
- Runtime optimization has `eng-kernel-ir-v1` plus an interpreter executor for
  element-wise TimeSeries arithmetic, TimeSeries statistics reductions,
  trapezoid integration, scalar residual, finite-difference Jacobian, and
  Newton-step correctness tests.
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
  u` metadata into an executable scalar RHS `KernelIr`; fixed-step simulation
  remains on the normal runtime solver path.
- CLI example smoke checks kernel candidates, interpreter executor fallback
  metadata, component residual kernel candidates, state-space RHS benchmark
  target coverage, and native-backend non-availability without making a speedup
  claim.
- `report_spec.json`, `report.html`, and the IDE Kernel panel surface the
  selected backend, kernel candidates, executor status, and fallback reason as
  inspection metadata without presenting acceleration.
- `eng.exe jit-bench` remains a normal-runtime timing harness and makes no
  speedup claim; its `benchmark_targets` field records which checklist target
  families were observed in the current source's kernel plan.

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
- IDE panels expose system equation dependency rows from `system_ir` so variable
  dependencies and derivative states are visible without raw JSON.
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
- `StateTrajectory::time_value_points` keeps SolverOutput state trajectories
  directly convertible to TimeSeries-style `(time, value)` points, and runtime
  system/component artifact adapters use that helper.
- Fixed-step method dispatch now lives behind the solver module as
  `FixedStepMethod`/`solve_fixed_step_ode`, so runtime materialization calls the
  solver API instead of carrying a local dispatch wrapper.
- The supported one-state thermal runtime path now delegates RHS evaluation and
  fixed-step execution to `solver::thermal::solve_first_order_thermal`; runtime
  materialization only recognizes the system shape and prepares canonical input.

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

## State-Space

Title: `state-space: close stable workflow boundaries after vector runtime seed`

Definition of Done:

- Keep the vector runtime seed clearly scoped until the stable workflow
  boundary is decided.
- Current runtime covers `StateVector`, `InputVector`, and `LinearOperator`
  metadata, operator row/column checks, non-rectangular matrix diagnostics,
  unsupported unitful matrix-entry diagnostics, inverse-time coefficient checks
  and per-second canonicalization for derivative-compatible source units,
  report/review/IDE canonical operator matrix and named-entry summaries,
  continuous and discrete A/B execution, multi-state fixed-step Euler/RK4
  trajectories, state trajectory TimeSeries, `OutputLayout` preservation across
  solver input/result contracts, and solver-inspector metadata for states,
  inputs, outputs, timestep, tolerance, iterations, convergence, and failure
  reason. Plot/report output, IDE inspector support, and continuous/discrete
  state-space smoke fixtures are in place for the current seed scope.
- Remaining supported-workflow work includes broader operator algebra and
  coefficient-unit policy, and public stability wording.
- No nonlinear/DAE/adaptive or component-coupled solver claim is made.

## Nonlinear / DAE Solver

Title: `solver: integrate nonlinear and DAE paths beyond standalone algorithm seeds`

Current coverage:

- Runtime has a standalone damped Newton algorithm seed with finite-difference
  fallback, supplied analytic/JIT Jacobian hook, largest-residual summary,
  residual-history, convergence-status, singular-Jacobian, invalid-option, and
  nonconvergence tests.
- Runtime also has dense linear and fixed-point algorithm seeds.
- Runtime has a standalone implicit-Euler DAE seed over
  `F(x, xdot, z, u, t, p)` with optional mass matrix, initial-condition
  consistency checks, algebraic-variable initialization, ODE residual,
  algebraic-variable, and mass-matrix tests.
- Runtime has a standalone dynamic-component explicit-Euler seed that solves
  algebraic variables with fixed point at each timestep, updates state
  trajectories, carries algebraic trajectories through the common
  `SolverResult` output contract, and returns timestep-level convergence/failure
  diagnostics. Component solver result artifacts can now carry those
  state/algebraic trajectories through report spec, HTML, `.engres`, and the
  IDE assembly summary.

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

- Runtime has a delay buffer seed with linear and previous-sample interpolation
  policies, explicit initial-history policy, relationship artifacts,
  out-of-order/history-underflow diagnostics, and solver-style behavior-node
  evaluation tests.
- Component local expressions now diagnose invalid `delay(signal, duration)`
  calls, including missing arguments, unknown component port variables, and
  non-duration delay values. Example-smoke fixtures cover each public
  component behavior diagnostic code.
- Component local Predictor and external behavior expressions now diagnose
  invalid seed syntax and unknown component port variables before they become
  behavior-node metadata.
- Runtime has a Predictor behavior contract wrapper with input/output
  quantity-unit metadata, valid-range warnings, provenance/model hash,
  differentiability flag, solver Jacobian policy, and evaluation/failure tests.
- Runtime has an external function/process behavior wrapper with typed
  input/output contracts, provenance hash, determinism metadata, safe/repro
  profile policy, range warnings, adapter failure propagation, and tests.
- Component artifacts distinguish delay calls as
  `delay_call_runtime_buffer_seed_not_integrated` and Predictor calls as
  `predictor_call_contract_seed_not_integrated`; external behavior expressions
  report `external_behavior_wrapper_seed_not_integrated`.
- Review/report/component-graph artifacts and the IDE Assembly panel expose
  behavior nodes with source navigation metadata.
- `examples/internal/25_component_behavior_nodes` exercises valid delay,
  Predictor, and external adapter behavior-node artifacts in the CLI example
  smoke path.

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
- Numeric component graph solving remains Planned.

Current coverage:

- IDE Assembly panel renders component graph components, ports, connections,
  labels, statuses, and source-line navigation from `component_graph`.
- IDE Assembly panel renders behavior nodes plus generated equation and
  evaluated residual rows from assembly artifacts.
- Internal boundary fixtures can promote component-local `port.signal = literal`
  expressions into boundary residual equations and solve square dense linear
  residual graphs with explicit variable, residual, and RHS artifacts.
- `examples/official/21_thermal_component_assembly` exercises the focused
  Thermal assembly path with generated connection equations, component-boundary
  RHS equations, a square residual graph, and dense linear solver artifacts.
- `examples/internal/23_component_boundary_singular` exercises the same square
  residual graph path when the dense matrix is singular and requires a
  `linear_solve_failed`/`E-LINEAR-SINGULAR` artifact.
- `examples/internal/24_component_boundary_overdetermined` exercises the
  overdetermined residual graph limitation path and requires a
  `not_solved_overdetermined`/`W-ASSEMBLY-OVERDETERMINED-SEED` artifact instead
  of attempting a dense solve.
- Runtime has an internal dynamic-component solver seed, but component graph
  assembly is not yet lowered into that numeric path.

Title: `assembly: harden generated equations and residual graph artifacts`

Definition of Done:

- Collect component instances, ports, connection sets, and generated connection
  equations.
- Preserve component-local boundary equation seeds with RHS literals for
  internal square algebraic fixtures.
- Record state/algebraic/input/output classification, equation count, unknown
  count, residual list, dependency graph, algebraic-loop seed, sparsity
  placeholder, and solver-plan placeholder.
- Report and IDE show generated equations, source-line links, generated
  reasons, residual values, normalized residuals, scale policy, and residual
  graph metadata.
- IDE Assembly panel shows residual dependency rows from
  `assembly_summary[].residual_graph.dependencies`.
- Under/overdetermined cases produce diagnostics or limitation artifacts.
- No production multi-domain solver claim is made.
