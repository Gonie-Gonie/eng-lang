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
  -> HTML report + review.json
```

The repository implements this path incrementally. v0.4-preview is the first version where `eng run` writes bytecode, decodes it, executes a native VM seed, and writes `result.engres` from the VM execution record. v0.5-preview adds TimeSeries/statistics metadata to the same path. v0.6-preview adds PlotSpec v1, SVG rendering from PlotSpec, and a plot manifest.

## Crates

```text
eng_cli
  User-facing `eng.exe` command surface.

eng_compiler
  Source check, diagnostics, AST, semantic metadata, entry selection data, and bytecode v1.

eng_runtime
  Run/build orchestration, bytecode VM seed, object store, result.engres generation, and doctor checks.

eng_report
  SVG and HTML review artifact generation.

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
lazy summary cache metadata
integrate(HeatRate over Time) -> Energy metadata
```

v0.6-preview adds:

```text
PlotSpec v1
line plot model
axis unit labels
SVG export from PlotSpec
plot manifest
```

The VM object store currently supports:

```text
scalar
table
timeseries
array
```

Schema columns remain public boundary metadata. They are not emitted as runtime scalar objects.

## Data Boundary

Foreign data crosses into EngLang through schemas:

```text
CSV source
  -> schema
  -> header validation
  -> source hash provenance
  -> typed table object seed
```

v0.5 builds TimeSeries/statistics metadata on top of this boundary. Numeric kernels remain deferred.

## Reviewability

Every run produces human-readable artifacts:

```text
build/<stem>.engbc
build/result/result.engres
build/result/review.json
build/result/report.html
build/result/plots/timeseries.svg
build/result/plots/plot_spec.json
build/result/plots/plot_manifest.json
```

These artifacts carry:

```text
source hash
bytecode hash
compiler version
runtime version
entry metadata
schema/CSV provenance
diagnostics
typed binding summaries
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
