# Roadmap

This roadmap follows the v9 master plan. v9 does not reverse the v8 language decisions; it reorganizes development into concrete version-by-version execution targets.

The active rule for contributors is:

```text
Pick the target version first, then read the detailed design chapters.
```

## Current Status

```text
v0.1-preview  complete and tagged
v0.2-preview  complete and tagged, with v9 backfill in progress
v0.3-preview  next target
```

## v0.1-preview — Repository, CLI, Parser, Unit Seed

Goal:

```text
Create a reproducible Rust repository with a Python-free preview path,
basic CLI, parser/frontend foundation, diagnostics, and unit registry seed.
```

Required outputs:

```text
- Rust workspace
- repo-local Windows setup through dev.bat
- eng.exe command skeleton
- doctor/check/run/build/view/new/test
- source span model
- lexer/parser skeleton
- typed AST skeleton
- diagnostic foundation
- stdlib/prelude.eng
- stdlib/units.eng
- .engbc preview artifact
- .engres preview artifact
- review.json/report.html/SVG artifact skeleton
- official smoke examples
```

Release gate:

```text
[x] dev.bat setup works
[x] dev.bat ci works
[x] eng.exe doctor works
[x] eng.exe check works
[x] eng.exe run creates preview artifacts
[x] no Python dependency in core path
```

v9 backfill:

```text
[ ] explicit compiler UnitInfo seed structure
```

## v0.2-preview — Semantic Analysis, Fast `=`, Quantity Rules

Goal:

```text
Make local fast declarations, quantity inference, expected type metadata,
dimensionless rules, and ambiguity diagnostics visible to compiler tooling.
```

Required outputs:

```text
- symbol/semantic skeleton
- fast `=` local declaration
- no `:=` diagnostic
- dimensionless concept
- dimensionless + physical operation errors
- ambiguous quantity warning
- ExpectedType structure
- TypeInfo structure
- UnitDerivation structure
- InferredDeclaration record
- quantity completion data table
- hover hint data
```

Release gate:

```text
[x] `L = 1 m + 20 cm` is accepted
[x] `X = 1 m + 20` errors
[x] `Q = 1 + 2 kW` errors
[x] `Q := 10 kW` errors
[x] `power = 10 kW` warns
[x] hover hint data is generated
[x] expected type data is generated
[ ] TypeInfo and UnitDerivation are explicit review records
```

## v0.3-preview — Schema, Promote, CSV Data Boundary

Goal:

```text
Bring external CSV data into the typed Eng world through schema validation.
```

Required outputs:

```text
- schema block AST
- schema symbol table
- promote csv expression check
- CSV reader
- DateTime index seed
- column type/unit validation
- missing policy seed
- constraint seed
- source file hash provenance
- CSV header metadata for tooling
```

Release gate:

```text
[ ] promote csv works
[ ] missing required column error
[ ] source file provenance recorded
[ ] schema diagnostics have source spans
```

## v0.4-preview — Bytecode VM, Entry-based Run, Result File

Goal:

```text
Run `.eng` through bytecode and VM without Python.
```

Required outputs:

```text
- .engbc bytecode v1
- eng VM seed
- object store
- scalar/table values
- script main(args: Args) entry execution
- result.engres v1
- entry not found diagnostic
```

## v0.5-preview — TimeSeries, Statistics, Lazy Summary

Goal:

```text
Make EngLang usable for first engineering data analysis workflows.
```

Required outputs:

```text
- TimeSeries[Time] type
- axis metadata
- mean/max/min/p95 seed
- integrate(HeatRate over Time) -> Energy
- lazy summary cache
- HeatRate sum lint
```

## v0.6-preview — PlotSpec, Interactive-friendly Plotting, SVG Export

Required outputs:

```text
- PlotSpec v1
- line plot
- axis unit labels
- interactive-friendly plot data model
- SVG export from PlotSpec
- plot manifest
```

## v0.7-alpha — Basic Report and Review Artifact

Required outputs:

```text
- review.json schema
- report data model
- variable table
- inferred declaration table
- unit conversion table
- schema summary
- plot manifest section
- warning list
```

## v0.8-alpha — Minimal `system` and `eq`

Required outputs:

```text
- system block
- parameter/state/input
- equation block
- eq relation
- der()
- equation unit check
- simple residual representation
- diagnostic when physical equations use ==
```

## v0.9-alpha — Portable Demo Hardening and Packaged Execution Candidate

Required outputs:

```text
- Windows portable zip
- official CSV+plot example
- official simple system example
- path tests for Korean and space-containing paths
- no Python/Rust install required for packaged preview
```

## v1.0-stable — Stable Core Release

v1.0 must provide all four:

```text
1. typed data analysis
2. plotting/report
3. minimal system/equation
4. packaged standalone execution
```

Release gate:

```text
[ ] official examples pass
[ ] spec code blocks check
[ ] docs complete for supported features
[ ] portable zip smoke test
[ ] report/review generated
[ ] version/format headers present
```

## v1.1 — Uncertainty Core

Required outputs:

```text
- Measured[T]
- Interval[T]
- Distribution[T] seed
- Ensemble[T] seed
- uncertainty metadata
- simple propagation
- distribution summary/plot
```

## v1.2 — Data-driven Modeling and Basic ANN

Required outputs:

```text
- eng.ml package
- regression
- basic ANN/MLP
- train/test split
- RMSE/MAE/R2
- residual/parity plot
- model card
- leakage lint
```

## v1.3 — LSP and VS Code Extension

Required outputs:

```text
- eng-lsp.exe
- VS Code extension preview
- syntax highlighting
- diagnostics
- hover type/unit
- basic completion
- schema column completion seed
- run/check/open report commands
```

## v1.4 — Tester IDE Maturity and JIT Start

Required outputs:

```text
- tester IDE file open/save
- check/run panels
- diagnostics panel
- variable/unit conversion table
- plot/review preview
- hot kernel detection
- numeric kernel lowering interface
```

## v1.5 — Standalone/AOT Maturity

Required outputs:

```text
- eng.exe build --standalone
- model.exe or packaged runner
- engpkg maturity
- Args-based CLI help
- runtime bundling
- lock file
- repro profile
```

## v2.0 — Open Domain/Port, Component Ecosystem, Advanced Platform

Required outputs:

```text
- open domain/port system
- user-defined domain
- across/through variables
- conservation contract
- Fluid[Medium]
- MechanicalNode[Frame, Axis]
- component/connect
- connection summary report
- multi-domain warnings
- uncertainty/optimization maturity
- native JIT/AOT maturity
- domain package ecosystem
```

## Work Breakdown Rules

Issues should include a version target whenever possible.

```text
language/*
compiler/*
runtime/*
numeric/*
plot/*
report/*
tooling/*
docs/*
examples/*
release/*
```

Recommended size:

```text
1 issue = 1-5 commits
1 PR = 1 issue or 2-3 closely related issues
1 version = 20-60 commits
```

Rough estimates from the v9 plan:

```text
portable demo: 80-150 commits
v1.0: 180-300 commits
v2.0: 350-600 commits
```
