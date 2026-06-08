# Development Tracks

Tracks are long-term capability areas. They are not public release versions.

## T1 Core Language

Current preview scope:

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

Current preview scope:

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

Current preview scope:

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

Current preview scope:

```text
- system block
- eq relation
- der()
- one-state thermal system metadata
- explicit solver-boundary artifacts
```

Deferred:

```text
- multi-state nonlinear solving
- adaptive solvers
- general equation-system runtime
```

## T5 IDE / LSP

Current preview scope:

```text
- native tester IDE
- UI settings, light/dark theme, font/layout controls
- syntax highlighting, diagnostics, completions
- PlotSpec preview and runtime summaries
- experimental eng-lsp.exe smoke/snapshot path
- packaged VS Code extension preview
```

Deferred:

```text
- full persistent LSP editor integration
- quick fixes
- production-grade IDE project model
```

## T6 Uncertainty

Implementation seeds on `main`:

```text
- measured values
- intervals
- distributions
- deterministic samples
- propagation metadata
- histogram artifact path
```

Not yet public-supported:

```text
- full Monte Carlo semantics
- Jacobian propagation
- broad unit conversion inside samples
- stable uncertainty language contract
```

## T7 Data-Driven Modeling

Implementation seeds on `main`:

```text
- train/test split metadata
- regression/basic MLP path
- source and argument diagnostics
- RMSE/MAE/R2
- model card metadata
- parity/residual plots
```

Not yet public-supported:

```text
- general ML package semantics
- broader algorithms
- stable model artifact contract
```

## T8 Runtime Optimization / JIT / AOT

Implementation seeds on `main`:

```text
- eng_jit crate
- eng.exe jit-plan
- eng.exe jit-bench
- interpreter baseline metadata
- backend selection metadata
- hot-kernel candidate estimates
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
- IDE Domain Graph inspection
- LSP completion/hover metadata
- domain contract and compatibility diagnostics
```

Not yet public-supported:

```text
- numeric multi-domain simulation
- domain package registry
- open component ecosystem
```

## T10 Class / Domain Object

Planned scope:

```text
- class declaration for typed engineering objects
- object literal and field access
- default values and validation
- immutable copy-with update
- report/review serialization
- IDE field completion and object summary
- class object as system/component parameter
```

Non-goals:

```text
- deep inheritance
- hidden mutable global state
- class as replacement for system/component
- port/connect inside class
```

## T11 General Programming / Side Effects

Design policy in `v0.2-preview`:

```text
- file/path/process/network concepts are typed
- side effects are explicit
- environment/time dependencies are visible
- report/review can record external effects
- safe/normal/repro profiles define allowed side-effect envelopes
```

Planned implementation order:

```text
1. eng.path path types and helpers
2. exists and environment dependency metadata
3. read text/json/toml with source hashes
4. write/export hardening and output manifest
5. copy/move/delete with explicit confirmation
6. log/warn/run-log artifacts
7. run command and ProcessResult
8. test/assert/golden support
```

Deferred:

```text
- broad filesystem mutation
- network/download
- process sandboxing
- full filesystem permission model
- package registry
```
