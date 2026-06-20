# Development Tracks

Tracks are long-term capability areas. They are not public release versions.

## T1 Core Language

Current supported scope:

```text
- fast `=`
- no `:=`
- dimensionless diagnostics
- top-level file execution as the default workflow
- root `args { ... }` as the only args declaration syntax
- importable top-level `const`
- pure scalar `fn` definitions with function-local bindings
- relative file imports for importable declarations
- system/equation syntax seeds
- source-preserving `eng fmt` for current block syntax and official examples
```

Deferred:

```text
- broader expression language
- package/module import system
- multi-return functions
- full AST-aware formatter policy
- stable breaking-change policy
- full language editioning
```

## T2 Data Boundary

Current supported scope:

```text
- schema/promote
- CSV import
- DateTime index metadata
- missing policy seed
- typed `args { ... }` primitives and path defaults
```

Deferred:

```text
- general table formulas
- richer data source types
- quantity/unit-literal Args
```

## T3 Statistics, Plot, And Report

Current supported scope:

```text
- TimeSeries statistics
- integrate(... over Time)
- unit-aware print interpolation
- explicit one-row summary CSV export
- PlotSpec v1
- SVG rendering
- review.json and report.html artifacts
```

Deferred:

```text
- multi-series and interactive plot semantics
- richer report layout
- general quantity-aware kernels
- first-class Summary objects
```

## T4 System / Equation

Current supported scope:

```text
- system block
- eq relation
- der()
- one-state thermal system metadata
- explicit solver-boundary artifacts
- source-equation fixed-step and adaptive Heun ODE workflows with scalar or
  Time-indexed TimeSeries inputs
- typed-block discrete and continuous state-space fixed-step workflows with
  `states`/`inputs` blocks, `StateVector[...]`, `InputVector[...]`, and
  operator declarations
```

Internal runtime seeds:

```text
- standalone dense linear, solver-API fixed-point, and solver-API damped
  Newton algorithms, with CLI smoke coverage for linear residual graph
  convergence/singular failure artifacts plus solver-API and source
  `solve component_graph` fixed-point convergence and nonconvergence failure
  artifacts
- SolverInput/SolverResult state, input, parameter, finite numeric value, and validated output layout contracts
- dense linear solver seeds reject non-finite matrix/RHS values and invalid tolerances
- Newton solver seeds reject non-finite initial guesses
- Newton iteration failures for singular linear solves and failed line-search
  candidates are returned as failure artifacts
- solver residual diagnostics use a shared scaled Euclidean norm helper
- fixed-point, Newton, and dense linear solver seeds reject non-finite intermediate values
- ResidualGraph linear assembly rejects non-finite coefficients and RHS values
- ResidualEvaluator rejects non-finite inputs, scales, values, normalized values, and norms
- DAE seeds reject non-finite inputs/parameters, derivative, mass-application,
  and algebraic-init values
- fixed-step ODE CLI smoke covers two-state explicit Euler/RK4 trajectories,
  final partial timestep handling, and non-finite RHS/update failure artifacts
- solver-API adaptive Heun/Euler ODE seed with fixed output-grid trajectories,
  internal substep adaptation, accepted/rejected substep reports, and
  step-limit/non-finite failure diagnostics; adaptive system runtime/report
  artifacts preserve those reports as `step_diagnostics`
- one-state thermal `simulate` integration for `solver = adaptive_heun`,
  including optional numeric `tolerance`, explicit `duration`, fixed output
  TimeGrid artifacts, and internal fixture/CLI smoke coverage
- source-equation `simulate` integration for `solver = adaptive_heun` through
  the shared `SourceRhsEvaluator`, with fixed output-grid trajectories,
  adaptive internal substeps, scalar inputs, and TimeSeries input materialization
- source-equation RHS evaluation pre-parses derivative coefficients and RHS
  expressions through the shared arithmetic expression parser while preserving
  unit-literal metadata
- internal continuous state-space `simulate` integration for
  `solver = adaptive_heun` on shape-checked `der(x) eq A * x + B * u`
  systems, with fixed output TimeGrid trajectories and adaptive internal
  substeps
- fixed-step ODE and dynamic-component updates use the actual final partial TimeGrid interval
- explicit Euler samples RHS values at the start of each fixed-step interval
- fixed-step ODE, fixed-point, and dynamic-component seeds reject non-finite RHS/update values
- state-space RHS/discrete seeds reject non-finite matrix, sampled-input, derivative, and updated-state values
- state-space non-rectangular matrix diagnostics, non-numeric/non-finite and
  unsupported unitful coefficient diagnostics, and inverse-time
  derivative-coupling coefficient canonicalization
- supplied analytic/JIT Jacobian hook for Newton
- solver-API standalone implicit-Euler DAE seed over F(x, xdot, z, u, t, p)
- optional DAE mass matrix and initial consistency checks
- explicit DAE method policy where BDF requests return
  E-DAE-METHOD-UNSUPPORTED until a real BDF implementation exists
- narrow source `solve component_graph` Newton bridge for coupled multi-variable unitful HeatRate
  nonlinear residuals, with finite-difference Jacobian by default,
  `source_linear_terms` Jacobian hook, residual history, and largest-residual
  artifacts
- narrow source `solve component_graph` implicit-Euler DAE bridge with assembly
  state/algebraic split, `DaeInput` generation, algebraic initialization,
  identity mass-matrix fallback, trajectories, step diagnostics, and failure
  artifacts
- standalone dynamic-component explicit-Euler seed with algebraic-free state updates, algebraic solve per timestep, and common SolverResult state/algebraic trajectories
- derivative-form ResidualGraph to dynamic-component RHS bridge for algebraic-free dynamic seeds
- residual-graph explicit-Euler dynamic-component entrypoint with layout
  validation and common SolverResult timestep diagnostics
- residual-graph semi-implicit dynamic-component entrypoint with per-timestep
  dense linear algebraic residual solves and failure diagnostics
- EquationAssembly dynamic-component state/algebraic/input/parameter split validation into solver layouts
- internal EquationAssembly-to-dynamic-component bridge that lowers simple
  arithmetic-linear derivative/algebraic residuals into explicit/semi-implicit solver seeds
  and preserves component artifact equation/unknown counts
- component solver result trajectory, timestep-diagnostic, and per-step
  nonconvergence failure-artifact adapter for internal dynamic-component
  SolverResult output
- solver-API delay buffer with interpolation and initial-history policies
- solver-API delay node adapter that feeds delayed values into fixed-step RHS evaluation
- component-local delay(signal, duration) diagnostics for port, prior local, and nested delay signals
- component-local Predictor and external behavior signal diagnostics for port, prior local, and nested delay signals
- solver-API Predictor contract wrapper with model hash, range warnings, and Jacobian policy
- solver-API Predictor node adapter that feeds predictor outputs into fixed-step RHS evaluation
- solver-API external behavior wrapper with provenance, profile policy, and failure propagation
- solver-API external behavior adapter that feeds external outputs into fixed-step RHS evaluation
- solver-API behavior graph RHS adapter that chains delay, Predictor, and external behavior nodes from state/input/parameter/prior-node signals
- component graph/report/IDE behavior nodes for delay, Predictor, and external calls with inferred contract fields and diagnostic channels
- narrow source `solve component_graph` integration for algebraic-free
  unitful temperature explicit-Euler component RHS evaluation with delay,
  deterministic Predictor identity-wrapper, and deterministic external adapter
  identity-wrapper behavior nodes
```

Deferred:

```text
- broad language-integrated nonlinear/DAE solving beyond the narrow component
  residual source smokes
- broad language-integrated dynamic component graph solving beyond the
  simple-linear source path
- broad language-integrated delay/Predictor/external behavior graph solving
  beyond the narrow unitful temperature explicit-Euler source behavior RHS smokes
- broad adaptive solvers beyond the source-equation, one-state thermal, and
  internal continuous state-space `adaptive_heun` paths
- general equation-system runtime beyond the supported one-state thermal and
  source-equation fixed-step/adaptive shapes
- broad state-space operator algebra, nonlinear/DAE state-space coupling,
  discrete adaptive state-space, and component-coupled state-space solving
```

## T5 IDE / LSP

Current stable tooling scope:

```text
- Tauri/WebView tester IDE
- docked explorer/editor/problems/terminal layout with Variables/Plot/Run inspector tabs
- diagnostics and caret completions
- PlotSpec viewer beside runtime variable summaries
- solver inspector/result summaries for system state trajectories, component
  solver state/algebraic trajectories, input/output series metadata, timestep,
  tolerance, iterations, convergence, failure reason, and component solver
  failure code/reason metadata on TimeSeries rows
- system and residual dependency graph inspector tables
- internal eng-lsp.exe smoke/snapshot path
- packaged VS Code extension source and VSIX
```

Deferred:

```text
- full persistent LSP editor integration
- quick fixes
- production-grade IDE project model
```

## T6 Uncertainty

Internal implementation seeds on `main`:

```text
- measured values
- intervals
- distributions
- deterministic samples
- propagation metadata
- histogram artifact path
```

Planned:

```text
- full Monte Carlo semantics
- Jacobian propagation
- broad unit conversion inside samples
- stable uncertainty language contract
```

## T7 Data-Driven Modeling

Internal implementation seeds on `main`:

```text
- train/test split metadata
- regression/basic MLP path
- source and argument diagnostics
- RMSE/MAE/R2
- model card metadata
- parity/residual plots
```

Planned:

```text
- general ML package semantics
- broader algorithms
- stable model artifact contract
```

## T8 Runtime Optimization / JIT / AOT

Internal implementation seeds on `main`:

```text
- eng_jit crate
- eng.exe jit-plan
- eng.exe jit-bench with benchmark target coverage metadata and deterministic
  interpreter-kernel sample executions for CSV, multi-statistics, residual,
  component, and state-space smoke targets, with dev-gate assertions for each
  checklist benchmark target family
- benchmarks/B01_csv_heat_rate through B06_nonlinear_solver with local input
  data, EngLang source, expected target metadata, timing policy, correctness
  fragments, and no-speedup-claim comparison policy
- interpreter baseline metadata
- backend selection metadata
- hot-kernel candidate estimates
- report-spec/report.html and IDE Kernel panel inspection of selected backend,
  candidates, executor status, and fallback reasons
- interpreter kernel IR and executor correctness tests for arithmetic,
  statistics, integration, residual, state-space RHS, Jacobian, Newton-step, and
  explicit-Euler solver-step kernels
- checked TimeSeries arithmetic, statistics, and integration metadata lowering into
  executable KernelIr
- component assembly residual graph lowering into scalar residual KernelIr
- square component residual Jacobian kernel-plan candidates backed by
  finite-difference execution over scalar residual KernelIr
- square component Newton-step kernel-plan candidates backed by the dense
  solver-step interpreter kernel
- continuous state-space A/B RHS lowering into scalar KernelIr
- state-space explicit-Euler solver-step kernel-plan candidates backed by the
  RHS interpreter KernelIr
- per-candidate executor/fallback reason metadata
```

Not yet public-supported:

```text
- native code generation
- runtime acceleration claim
- optimized model.exe/AOT output
```

## T9 Domain / Component

Supported scoped slice:

```text
- constrained Thermal component boundary assembly in
  examples/official/23_thermal_component_assembly
- source-to-solver linear Thermal algebraic graph in
  examples/official/24_linear_algebraic_thermal_node
- source-to-solver fixed-point ResidualGraph loop in
  examples/official/25_fixed_point_loop
- source-to-solver simple-linear dynamic component room in
  examples/official/26_dynamic_component_room
- source-to-solver coupled nonlinear residuals in
  examples/official/27_nonlinear_algebraic
- source-to-solver multi-state unitful temperature DAE in examples/official/28_small_dae
- source-to-solver unitful temperature delay/Predictor/external behavior RHS smokes in
  examples/official/29_delay_component_solver,
  examples/official/30_predictor_component_solver, and
  examples/official/31_external_behavior_solver
- constrained Thermal/Fluid[Water] pressure/flow algebraic residual solve in
  examples/official/32_small_thermal_fluid_loop with generated connection
  equations, component-local boundary seeds, pipe pressure/flow equations, dense
  linear residual solve artifacts, and largest-residual reporting
- source-to-solver unit-parameterized Thermal wall residual solve in
  examples/official/33_unit_parameterized_wall with Conductance [W/K]
  parameters converted into compatible HeatRate residual display units
- system-local name = Component(...) instances with empty constructors or declared numeric parameter defaults/overrides
- connect instance.port to instance.port
- generated connection equations plus literal boundary seeds, simple
  component-local equations, and unit-parameterized linear coefficient equations
  such as Conductance * TemperatureDelta across compatible residual display units
- square dense linear residual solve artifact
- explicit `solve component_graph` fixed-point artifact with tolerance,
  max-iteration, relaxation, initial-guess, convergence, and failure metadata
- explicit/semi-implicit dynamic `solve component_graph` artifacts with
  timestep, duration, initial state, trajectories, step diagnostics, and
  algebraic failure metadata
- `solver = newton` component residual artifacts with residual history,
  finite-difference Jacobian default, optional `source_linear_terms` Jacobian
  hook, and nonconvergence failure metadata
- `solver = implicit_euler_dae` component residual artifacts with
  state/algebraic split, algebraic initialization, identity mass-matrix
  fallback, state/algebraic trajectories, step diagnostics, and inconsistent
  initial-condition failure metadata
- explicit-Euler behavior graph component artifacts with integrated behavior
  node status, typed contracts, state trajectory, and per-step behavior graph
  diagnostics
```

Implementation seeds on `main`:

```text
- user-defined domain declarations
- across/through variables
- conservation metadata
- component ports
- generic domain parameters
- connection review/report metadata
- connection-set assembly metadata
- generated connection-equation and residual graph placeholders
- equation/unknown count metadata
- generated-equation reasons and normalized residual evaluation artifacts
- domain-plan based multi-domain metadata
- constrained Thermal/Fluid/MechanicalNode boundary solve fixture
- constrained official Thermal/Fluid[Water] pressure/flow solve fixture
- connection constraint consistency artifacts
- IDE Domain Graph inspection
- IDE assembly equation/residual inspection
- IDE residual dependency graph inspection
- LSP completion/hover metadata
- domain contract and compatibility diagnostics
```

Not yet public-supported:

```text
- production numeric multi-domain simulation
- production pressure-drop packages
- broad args/object/non-arithmetic constructor bindings and nonlinear/general compound-unit
  component-local equation solving
- general boundary-condition/component-behavior solving
- domain package registry
- open component ecosystem
```

## T10 Class / Domain Object

Supported scope:

```text
- class declaration for typed engineering objects
- object literal and field access
- class validate blocks
- validation PASS/FAIL object artifacts
- zero-argument metadata methods with direct `self.<field>` returns
- immutable copy-with metadata
- report/review serialization
- IDE field completion and object summary
- LSP class/object hover and completion metadata
- class object as system/component parameter
```

Deferred:

```text
- method arguments and runtime dispatch
- runtime object dispatch/lowering
```

Non-goals:

```text
- deep inheritance
- hidden mutable global state
- class as replacement for system/component
- port/connect inside class
```

## T11 General Programming / Side Effects

Implemented seeds in the current package scope:

```text
- file/dir path defaults
- join/parent/stem/extension path helpers
- exists checks recorded as environment dependency provenance
- read text/json/toml UTF-8 raw string reads
- source hash provenance for read-only inputs
- write text/json output seed
- idempotent overwrite hardening for write/export outputs
- output_manifest.json for generated artifacts
- constrained copy/move/delete file operation seed
- confirm/recursive metadata requirements for destructive operations
- output manifest records for generated-output file operations
- print plus log debug/info/warn/error runtime message metadata
- run_log.json artifact records for saved runs
- run command external process seed
- ProcessResult typed binding and process_results.json records
- test/assert/golden workflow verification seed
- test_results.json records for saved runs
- review/result/report-spec environment_dependencies fields
```

Remaining design policy:

```text
- file/path/process/network concepts are typed
- side effects are explicit
- environment/time dependencies are visible
- report/review can record external effects
- safe/normal/repro profiles define allowed side-effect envelopes
```

Planned implementation order:

```text
1. eng.path path types and helpers [implemented]
2. exists and environment dependency metadata [implemented]
3. read text/json/toml with source hashes [implemented]
4. write/export hardening and output manifest [implemented]
5. copy/move/delete with explicit confirmation [implemented]
6. log level/run-log artifacts [implemented]
7. run command and ProcessResult [implemented]
8. test/assert/golden support [implemented]
```

Deferred:

```text
- broad filesystem mutation
- network/download
- process sandboxing
- full filesystem permission model
- package registry
```
