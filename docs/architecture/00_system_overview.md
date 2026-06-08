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

The current public preview implements this path incrementally: `eng run` builds
bytecode, decodes it, executes a native VM seed, and returns a typed result
object from the VM execution record. `--save-artifacts` writes the corresponding
`.engbc`, `.engres`, PlotSpec, SVG, report, and review files. The supported preview surface includes
TimeSeries/statistics metadata, PlotSpec v1, SVG rendering, plot manifests,
review/report artifacts, minimal physical `system` and `eq` metadata, runtime
table columns, TimeSeries points, computed summary statistics, trapezoidal
integration, CSV-derived PlotSpec points, and a reviewable system IR with a
computed fixed-step ODE preview for the official simple thermal system.

## Crates

```text
eng_cli
  User-facing `eng.exe` command surface.

eng_compiler
  Source check, diagnostics, AST, semantic metadata, top-level workflow data, and bytecode v1.

eng_jit
  Experimental hot-kernel detection and numeric lowering-plan metadata.

eng_runtime
  Run/build orchestration, bytecode VM seed, object store, result.engres generation, and doctor checks.

eng_report
  PlotSpec, SVG, ReportSpec, and HTML review artifact generation.

stdlib
  Repo-local prelude and unit registry seed.
```

## Current Runtime Boundary

Current bytecode/runtime path:

```text
source
  -> check_file
  -> build top-level workflow metadata
  -> build_bytecode
  -> parse_bytecode
  -> execute_bytecode
  -> result_json
```

Statistics/data preview adds:

```text
TimeSeries[Time] of HeatRate
axis metadata
computed mean/time_weighted_mean/median/std/p90/p95/duration_above summary values for the official CSV path
trapezoidal integrate(HeatRate over Time) -> Energy value
```

Plot/report preview adds:

```text
PlotSpec v1
line plot model
axis unit labels
SVG export from PlotSpec
plot manifest
```

Review/report preview adds:

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

System/equation preview adds:

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

Current system runtime preview adds:

```text
report_spec/result solver_boundary status = computed for the official one-state thermal ODE
explicit_euler_fixed_step ODE runner preview
solver_result trajectory in result.engres
```

Runtime optimization track planning adds:

```text
eng-kernel-plan-v1
hot-kernel candidates for TimeSeries arithmetic/statistics/integration
system residual interface seeds for future RHS/Jacobian kernels
backend = interpreter-fallback
```

The VM object store currently supports:

```text
scalar
table
timeseries
array
```

Schema columns remain public boundary metadata. They are not emitted as runtime scalar objects.

System variables are also boundary metadata. They appear in review/report
variable tables, system summaries, the system IR dependency list, and
solver_plan seed metadata. The current system runtime preview additionally
recognizes the official one-state thermal ODE and records a fixed-step runtime
preview in run artifacts.

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

The current data-boundary track parses official CSV DateTime and numeric
quantity columns into runtime pages. The supported coil heat-rate expression
path computes TimeSeries values, mean/time_weighted_mean/max/min/median/std/
pNN/duration_above kernels, and trapezoidal HeatRate-over-Time integration
without Python.

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
workflow metadata
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
