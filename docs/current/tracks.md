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
```

Deferred:

```text
- broader expression language
- package/module import system
- multi-return functions
- full formatter policy
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
```

Internal runtime seeds:

```text
- standalone dense linear, fixed-point, and damped Newton algorithms
- supplied analytic/JIT Jacobian hook for Newton
- standalone implicit-Euler DAE seed over F(x, xdot, z, u, t, p)
- optional DAE mass matrix and initial consistency checks
- delay buffer with interpolation and initial-history policies
- Predictor contract wrapper with model hash, range warnings, and Jacobian policy
- external behavior wrapper with provenance, profile policy, and failure propagation
```

Deferred:

```text
- language-integrated nonlinear/DAE solving
- language-integrated delay/Predictor/external behavior graph solving
- adaptive solvers
- general equation-system runtime
```

## T5 IDE / LSP

Current stable tooling scope:

```text
- Tauri/WebView tester IDE
- docked explorer/editor/problems/terminal layout with Variables/Plot/Run inspector tabs
- diagnostics and caret completions
- PlotSpec viewer beside runtime variable summaries
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
- eng.exe jit-bench
- interpreter baseline metadata
- backend selection metadata
- hot-kernel candidate estimates
- interpreter kernel IR and executor correctness tests for arithmetic,
  integration, residual, and Jacobian kernels
- per-candidate executor/fallback reason metadata
```

Not yet public-supported:

```text
- native code generation
- runtime acceleration claim
- optimized model.exe/AOT output
```

## T9 Domain / Component

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
- domain-plan based multi-domain metadata
- connection constraint consistency artifacts
- IDE Domain Graph inspection
- LSP completion/hover metadata
- domain contract and compatibility diagnostics
```

Not yet public-supported:

```text
- production numeric multi-domain simulation
- boundary-condition/component-behavior solving
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

Implemented seeds through `v1.0.0`:

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
