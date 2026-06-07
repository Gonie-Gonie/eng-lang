# Roadmap

This roadmap follows the v9 master plan. v9 does not reverse the v8 language decisions; it reorganizes development into concrete version-by-version execution targets.

The active rule for contributors is:

```text
Pick the target version first, then read the detailed design chapters.
```

## Current Status

```text
v0.1-preview  complete historical milestone
v0.2-preview  complete historical milestone, with v9 backfill
v0.3-preview  complete historical milestone
v0.4-preview  complete historical milestone
v0.5-preview  complete historical milestone
v0.6-preview  implemented on main
v0.7-alpha    implemented on main
v0.8-alpha    implemented on main
v0.9-alpha    implemented on main
v1.0-stable   latest stable baseline
v1.0.3        active release target: IDE/documentation hardening
v1.1          planned target; uncertainty code on main is experimental
v1.2          planned target; data-driven modeling code on main is experimental
```

Use [current status](current/status.md) and the
[feature maturity matrix](current/feature_maturity_matrix.md) as the
authoritative state layer. This roadmap describes version intent and required
outputs; it is not by itself a support claim.

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
[x] explicit compiler UnitInfo seed structure
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
[x] TypeInfo and UnitDerivation are explicit review records
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
[x] promote csv works
[x] missing required column error
[x] source file provenance recorded
[x] schema diagnostics have source spans
```

## v0.4-preview — Bytecode VM, Entry-based Run, Result File

Goal:

```text
Run `.eng` through bytecode and VM without Python.
```

Required outputs:

```text
[x] .engbc bytecode v1
[x] eng VM seed
[x] object store
[x] scalar/table/array value seed
[x] script main(args: Args) entry execution
[x] result.engres v1
[x] entry not found diagnostic
[x] bytecode encode/decode test
[x] VM scalar execution test
```

## v0.5-preview — TimeSeries, Statistics, Lazy Summary

Goal:

```text
Make EngLang usable for first engineering data analysis workflows.
```

Required outputs:

```text
[x] TimeSeries[Time] type
[x] axis metadata
[x] mean/max/p95 summary seed
[x] time_weighted_mean/median/std/pNN hardening kernels
[x] integrate(HeatRate over Time) -> Energy metadata
[x] lazy summary cache
[x] HeatRate sum lint
[x] TimeSeries VM object
[x] result.engres statistics/integration payload
```

## v0.6-preview — PlotSpec, Interactive-friendly Plotting, SVG Export

Required outputs:

```text
[x] PlotSpec v1
[x] line plot
[x] axis unit labels
[x] interactive-friendly plot data model
[x] SVG export from PlotSpec
[x] plot manifest
[x] eng view basic plot listing
```

## v0.7-alpha — Basic Report and Review Artifact

Required outputs:

```text
[x] review.json schema
[x] report data model
[x] variable table
[x] inferred declaration table
[x] unit conversion table
[x] schema summary
[x] plot manifest section
[x] warning list
```

Release gate:

```text
[x] review.json generated with review_schema_version
[x] report_spec.json generated with eng-report-spec-v1
[x] report data includes variables and unit conversions
[x] schema summary appears in review/report artifacts
[x] plot manifest path/hash appears in report_spec.json
[x] warning list appears in review/report artifacts
[x] official plotting example has review output
```

## v0.8-alpha — Minimal `system` and `eq`

Required outputs:

```text
[x] system block
[x] parameter/state/input
[x] equation block
[x] eq relation
[x] der()
[x] equation unit check
[x] simple residual representation
[x] diagnostic when physical equations use ==
```

Release gate:

```text
[x] system parses
[x] parameter/state/input variables appear in review/report variable tables
[x] eq checks unit consistency
[x] == diagnostic works
[x] simple residual representation appears in review.json/report_spec.json/result.engres
[x] simple system report shows equation summary
[x] official simple system example passes
```

## v0.9-alpha — Portable Demo Hardening and Packaged Execution Candidate

Required outputs:

```text
[x] Windows portable zip
[x] SHA256 checksum for the portable zip
[x] official CSV+plot example
[x] official simple system example
[x] path tests for Korean and space-containing paths
[x] no Python/Rust install required for packaged preview
```

Release gate:

```text
[x] .\dev.bat ci
[x] .\dev.bat package
[x] .\dev.bat package-smoke
[x] package-smoke extracts the zip under a path containing spaces and Korean characters
[x] packaged eng.exe doctor passes from the extracted folder
[x] packaged CSV+plot example produces result/report/PlotSpec artifacts
[x] packaged simple system example produces result/report artifacts
```

## v1.0-stable — Stable Core Release

v1.0 must provide all four:

```text
[x] typed data analysis
[x] plotting/report
[x] minimal system/equation
[x] packaged standalone execution
```

Release gate:

```text
[x] official examples pass
[x] CLI example smoke includes typed CSV, plot/report, and simple system examples
[x] docs complete for supported features
[x] portable zip smoke test
[x] portable package smoke builds and runs a standalone packaged runner
[x] report/review generated
[x] version/format headers present
[x] standalone .engpkg records package format, source, bytecode, hashes, and entry
[x] standalone lock records runtime/compiler/bytecode/result/report/plot versions
```

## v1.1 — Uncertainty Core

Before or alongside v1.1, address the v1.0 hardening register in
[docs/development/05_v1_0_gap_audit.md](development/05_v1_0_gap_audit.md).

Priority backfill:

```text
[x] docs-check command for supported docs/spec code blocks
[x] examples/official/01_csv_plot and examples/official/02_simple_system
[x] artifact schema/golden validation baseline
[x] Args metadata inventory for packaged runner help
```

Required outputs before v1.1 can be considered supported:

```text
- Measured[T]
- Interval[T]
- Distribution[T] deterministic path
- Ensemble[T] deterministic path
- uncertainty metadata in review/report/result artifacts
- simple propagation through source binding samples with scale/offset transform
  metadata
- propagation source terms in review/result/report/IDE artifacts
- distribution summary and PlotSpec histogram with bin metadata
- official uncertainty example and CLI smoke
- uncertainty source validation diagnostics
- diagnostics, IDE metadata, documentation, and release notes aligned
```

Status on `main`: experimental seed and official example support exist, but
this is not release-supported until the v1.1 gate is explicitly completed.

## v1.2 — Data-driven Modeling and Basic ANN

Required outputs before v1.2 can be considered supported:

```text
- eng.ml preview package surface in stdlib/eng/ml.eng
- regression deterministic path
- basic ANN/MLP deterministic path
- train/test split metadata and runtime counts
- RMSE/MAE/R2 metrics
- residual/parity plot paths
- model card
- leakage lint
- source validation diagnostics for split/model/evaluation links
- argument validation diagnostics for split/model/MLP options
- official data-driven modeling example and CLI smoke
- diagnostics, IDE metadata, documentation, and release notes aligned
```

Status on `main`: experimental seed and official example support exist, but
this is not release-supported until the v1.2 gate is explicitly completed.

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
