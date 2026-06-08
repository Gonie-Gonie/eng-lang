# Development Tracks

Tracks are long-term capability areas. They are not public release versions.

## T1 Core Language

Current preview scope:

```text
- fast `=`
- no `:=`
- dimensionless diagnostics
- script main(args)
- system/equation syntax seeds
```

Deferred:

```text
- broader expression language
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
- typed Args primitives
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
