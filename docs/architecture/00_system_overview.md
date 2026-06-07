# System Overview

EngLang's official execution path is:

```text
.eng source
  -> compiler front end
  -> typed semantic model
  -> .engbc bytecode
  -> native runtime / VM
  -> .engres typed result
  -> PlotSpec
  -> SVG plot
  -> HTML report + report_spec.json + review.json
```

The repository implements this path incrementally. v0.4-preview is the first version where `eng run` writes bytecode, decodes it, executes a native VM seed, and writes `result.engres` from the VM execution record. v0.5-preview adds TimeSeries/statistics metadata to the same path. v0.6-preview adds PlotSpec v1, SVG rendering from PlotSpec, and a plot manifest. v0.7-alpha hardens the review/report artifact contract with `report_spec.json`. v0.8-alpha adds minimal physical `system` and `eq` metadata. The current v1.0 hardening path materializes the official CSV example into runtime table columns, TimeSeries points, computed summary statistics, trapezoidal integration, CSV-derived PlotSpec points, and a reviewable system IR with an explicit unsolved solver boundary plus metadata-only solver_plan seeds.

## Crates

```text
eng_cli
  User-facing `eng.exe` command surface.

eng_compiler
  Source check, diagnostics, AST, semantic metadata, entry selection data, and bytecode v1.

eng_runtime
  Run/build orchestration, bytecode VM seed, object store, result.engres generation, and doctor checks.

eng_report
  PlotSpec, SVG, ReportSpec, and HTML review artifact generation.

stdlib
  Repo-local prelude and unit registry seed.
```

## Current Runtime Boundary

v0.4-preview:

```text
source
  -> check_file
  -> select entry
  -> build_bytecode
  -> parse_bytecode
  -> execute_bytecode
  -> result_json
```

v0.5-preview adds:

```text
TimeSeries[Time] of HeatRate
axis metadata
computed mean/time_weighted_mean/median/std/p90/p95/duration_above summary values for the official CSV path
trapezoidal integrate(HeatRate over Time) -> Energy value
```

v0.6-preview adds:

```text
PlotSpec v1
line plot model
axis unit labels
SVG export from PlotSpec
plot manifest
```

v0.7-alpha adds:

```text
review_schema_version
ReportSpec v1
variable table
inferred declaration table
unit conversion table
schema summary
plot manifest path/hash section
warning list
```

v0.8-alpha adds:

```text
system block
parameter/state/input variables
equation block
infix eq relation
der() dimension handling
equation unit consistency diagnostics
residual metadata in review/report/result artifacts
system_ir dependency metadata in review/report/result artifacts
solver_boundary status = unsolved
solver_plan metadata-only solve_order and Jacobian seed columns
```

The VM object store currently supports:

```text
scalar
table
timeseries
array
```

Schema columns remain public boundary metadata. They are not emitted as runtime scalar objects.

System variables are also boundary metadata in v0.8. They appear in review/report variable tables, system summaries, the system IR dependency list, and solver_plan seed metadata, but they are not lowered as executable VM scalar objects yet.

## Data Boundary

Foreign data crosses into EngLang through schemas:

```text
CSV source
  -> schema
  -> header validation
  -> source hash provenance
  -> typed table object
  -> runtime column pages
  -> TimeSeries points
```

The v1.0 hardening path parses official CSV DateTime and numeric quantity columns into runtime pages. The supported coil heat-rate expression path computes TimeSeries values, mean/time_weighted_mean/max/min/median/std/pNN/duration_above kernels, and trapezoidal HeatRate-over-Time integration without Python.

## Reviewability

Every run produces human-readable artifacts:

```text
build/<stem>.engbc
build/result/result.engres
build/result/review.json
build/result/report.html
build/result/report_spec.json
build/result/plots/timeseries.svg
build/result/plots/plot_spec.json
build/result/plots/plot_manifest.json
```

These artifacts carry:

```text
source hash
bytecode hash
report spec hash
compiler version
runtime version
entry metadata
schema/CSV provenance
diagnostics
typed binding summaries
variable/unit conversion/warning tables
system/equation/residual summaries
system IR dependencies
explicit solver boundary status
object store summary
```

## Deliberately Out of Core Path

The official core path must not use:

```text
X Python code generation
X Python runtime backend
X matplotlib report generation
X user-machine global toolchain assumptions
X axis=0/axis=1 as the public data model
```

Development helper scripts may use local tools, but `eng.exe run` must stay native and reproducible.
