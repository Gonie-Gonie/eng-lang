# Roadmap

This roadmap follows the current release policy:

```text
Public release labels describe packages.
Long-term capabilities are managed as tracks.
Solver maturity is claimed only by implemented source, compiler, runtime,
artifact, IDE, example, and test evidence.
```

Use [current status](current/status.md),
[version policy](current/version_plan.md), and the
[feature maturity matrix](current/feature_maturity_matrix.md) as the
authoritative state layer.

## Current Public Package

`v0.1.0` is the current published portable package line. It packages the
documented CLI/data/report workflow and curated user docs. It is not a general
solver release, and it does not include every newer solver-centered change on
`main`.

Public package scope:

```text
- eng.exe doctor/check/run/build/view
- typed CSV promote
- unit-aware HeatRate calculations
- TimeSeries statistics and integration metadata on the documented path
- PlotSpec/SVG output
- result/review/report artifacts
- explicit print/log/export/process/test/file-operation artifacts where covered
  by official examples
- standalone packaging for the official CSV workflow
- native tester IDE smoke path
- packaged eng-lsp.exe smoke/snapshot tooling
- curated user PDF and language grammar PDF
```

Known public-package boundary:

```text
- solver/system examples are scoped supported or internal fixtures unless their
  docs say otherwise
- no production nonlinear, DAE, adaptive, or broad multi-domain solver claim
- no native JIT/AOT speedup claim
- no stable persistent editor-service claim for LSP/VS Code
```

## Integrated Direction

The active philosophy is:

```text
System modeling produces typed TimeSeries.
Data analysis validates, calibrates, summarizes, and explains those TimeSeries.
```

That gives the long-term workflow:

```text
schema/promote
-> typed Table/TimeSeries
-> system/component simulation input
-> typed simulation output TimeSeries
-> metrics/validation/calibration
-> PlotSpec/report/review artifacts
-> IDE visual inspection
-> standalone package
```

Use [integrated language philosophy](current/philosophy.md) as the short-form
policy source.

## Active Target

Current `main` is focused on solver-centered implementation hardening:

```text
- keep public claims aligned with implemented scope
- keep official examples separate from internal fixtures
- add source-level solver examples only when they actually solve
- expose residual/RHS/failure artifacts in report/review/IDE surfaces
- keep native JIT work behind no-speedup-claim benchmark evidence
- keep release assets and tag state explicit
```

## Development Tracks

Tracks are described in [development tracks](current/tracks.md):

```text
T1 Core Language
T2 Data Boundary
T3 Statistics / Plot / Report
T4 System / Equation
T5 IDE / LSP
T6 Uncertainty
T7 Data-driven Modeling
T8 Runtime Optimization / JIT / AOT
T9 Domain / Component
T10 Class / Domain Object
T11 General Programming / Side Effects
```

A future public package may include early work from any track, but the track
name should remain separate from the release label and maturity status.

## IDE Direction

The tester IDE is a Tauri/WebView shell with a Rust backend and static
HTML/CSS/JS frontend:

```text
- Rust stays authoritative for compiler/runtime/report services.
- The UI uses HTML/CSS/JS for editor layout, docked panels, terminal,
  variable tables, and responsive plot/report inspection.
- The frontend is static-build friendly first, so the packaged IDE does
  not require Node on the target PC.
- Parser/check/run requests should be debounced and incremental enough for
  editor responsiveness.
- The packaged workflow should keep eng-ide.exe --smoke and dev.bat ide-check
  as user-visible validation paths.
```

This is a T5 IDE/LSP development-track item. It supports package smoke and
inspection workflows, but it is still not a full editor platform.

## Working Rule

Before claiming public support:

```text
1. Pick the public package scope or development track.
2. Add examples and diagnostics.
3. Add runtime/report/IDE metadata where relevant.
4. Update status, maturity, and user docs.
5. Pass the appropriate local gate.
```
