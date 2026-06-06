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

The repository implements this path incrementally. v0.4-preview is the first version where `eng run` writes bytecode, decodes it, executes a native VM seed, and writes `result.engres` from the VM execution record.

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

The VM object store currently supports:

```text
scalar
table
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

v0.5 will build TimeSeries/statistics behavior on top of this boundary.

## Reviewability

Every run produces human-readable artifacts:

```text
build/<stem>.engbc
build/result/result.engres
build/result/review.json
build/result/report.html
build/result/plots/timeseries.svg
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
