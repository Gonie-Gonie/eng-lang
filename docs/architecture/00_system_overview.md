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

The current runtime implements this path in two native layers. `eng run`
builds and executes the compact object-loading bytecode, then runtime
materialization evaluates typed tables, TimeSeries operations, workflow
modules, and supported numeric solver paths before producing artifacts.
`--save-artifacts` writes the corresponding `.engbc`,
`.engres`, PlotSpec, SVG, report, and review files. The supported
surface includes native HTTP/cache/table/sampling/case/model/SQLite workflow
slices, computed TimeSeries statistics and integration, uncertainty
propagation, PlotSpec/SVG rendering, and scoped system simulation and residual
solve results with convergence and failure evidence.

## Crates

```text
eng_cli
  User-facing `eng.exe` command surface.

eng_compiler
  Source check, diagnostics, AST, semantic metadata, top-level workflow data, and bytecode v1.

eng_jit
  Internal hot-kernel detection and numeric lowering-plan metadata.

eng_runtime
  Run/build orchestration, bytecode VM execution, object store, result.engres generation, and doctor checks.

eng_report
  PlotSpec, SVG, ReportSpec, and HTML review artifact generation.

stdlib
  Repo-local prelude and unit registry.
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
  -> materialize typed workflow and numeric runtime data
  -> result_json
```

Statistics/data support adds:

```text
TimeSeries[Time] of HeatRate
axis metadata
computed mean/time_weighted_mean/median/std/p90/p95/duration_above summary values for the official CSV path
trapezoidal integrate(HeatRate over Time) -> Energy value
```

Plot/report support adds:

```text
PlotSpec v1
line plot model
axis unit labels
SVG export from PlotSpec
plot manifest
```

Review/report support adds:

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

Compiler-time system/equation support adds:

```text
system block
parameter/state/input variables
equation block
infix eq relation
der() dimension handling
equation unit consistency diagnostics
residual metadata in review/report/result artifacts
system_ir dependency metadata in review/report/result artifacts
static solver_boundary status = unsolved
static solver_boundary reason = numeric solve has not been executed
static solver_plan status = ready
static solver_plan method = source_order_residual_plan
static ODE runner status = not_executed
source-order residuals and Jacobian sparsity dependencies
```

Runtime simulation and solve support adds:

```text
one-state thermal and multi-state source-equation ODE simulation
fixed-step Euler, RK4, and adaptive Heun trajectories
continuous/discrete typed-block state-space trajectories
dense linear, fixed-point, and Newton residual solves
implicit-Euler DAE and scoped dynamic-component solves
runtime solver boundary upgrades, convergence diagnostics, and failure artifacts
```

Runtime optimization track planning adds:

```text
eng-kernel-plan-v1
hot-kernel candidates for TimeSeries arithmetic/statistics/integration
component residual, finite-difference Jacobian, and Newton-step candidates
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

System variables begin as compiler-owned boundary metadata. They appear in
review/report variable tables, system summaries, the system IR dependency list,
and the static solver plan. Supported `simulate` and `solve`
commands then materialize native numeric results as typed trajectories,
residual values, convergence histories, step diagnostics, and explicit failure
artifacts. The static `unsolved` boundary means that no numeric result has
been produced yet; the `ready` plan and `not_executed` runner mean
that residual order and sparsity are available before runtime solver selection.

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
