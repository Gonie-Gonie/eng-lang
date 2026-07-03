# Bytecode VM and Result v1

The current package scope has an executable EngLang runtime path with
TimeSeries/statistics metadata, PlotSpec/SVG/manifest output, explicit
write/export outputs, constrained file operation records, output manifest
metadata, structured run-log messages, explicit process-result records, system
metadata, runtime test-result records, residuals, system IR dependencies,
metadata-only solver_plan seeds, an explicit solver boundary, and a fixed-step
ODE path for the official one-state thermal system:

```text
  .eng source
  -> compiler check
  -> top-level workflow metadata
  -> .engbc bytecode v1
  -> native VM seed
  -> result.engres v1
  -> PlotSpec v1
  -> SVG + plot manifest + run log + process results + test results + output manifest
  -> review/report/system artifacts
```

The VM is intentionally small, but it is a real execution boundary: `eng run`
builds bytecode, decodes it, executes the instruction stream, and returns the
result from the VM execution record. `--save-artifacts` writes the bytecode and
result objects to disk.

## Workflow Execution

File run/build always uses the source file's top-level workflow. Root
`args { ... }` metadata supplies CLI-bindable argument fields. `script` blocks
are rejected by the compiler and are not part of the execution model.

## Bytecode v1

Header:

```text
ENGBYTECODE 1
format = engbc-v1
bytecode_version = 1
compiler_version = 0.1.0
source_hash = ...
source_bytes = ...
source_lines = ...
tokens = ...
ast_items = ...
typed_bindings = ...
schemas = ...
csv_promotions = ...
workflow = top_level
workflow_args = args:Args
workflow_return = Report
args_schema = root args metadata, when declared
```

Object records:

```text
table|<binding>|<schema>|<row_count>|<source_hash>|<line>
scalar|<binding>|<quantity_kind>|<display_unit>|<line>
timeseries|<binding>|<axis>|<quantity_kind>|<display_unit>|<line>
array|<binding>|<element_type>|<len>|<line>
```

Instruction records:

```text
0000|enter_workflow|top_level
0001|load_table|sensor
0002|load_scalar|cp
0003|load_timeseries|Q_coil
0004|write_result|engres-v1
```

Current opcodes:

```text
enter_workflow
load_scalar
load_table
load_timeseries
load_array
write_result
```

## Object Store

The VM object store currently supports:

```text
scalar
table
timeseries
array
```

Schema columns and v0.8 system variables are public boundary metadata, not runtime scalar values. The bytecode builder skips those bindings and emits runtime objects only for executable top-level/script bindings and promoted tables.
TimeSeries objects carry axis, quantity, and display-unit metadata.

## Result v1

`result.engres` uses JSON:

```text
format = engres-v1
result_format_version = 1
bytecode_version = 1
workflow metadata
object_store summary
typed_payload
provenance
```

Plot/report support records:

```text
provenance.plot_spec_hash
provenance.report_spec_hash
```

System/equation support records:

```text
typed_payload.systems
typed_payload.solver_boundaries
typed_payload.system_ir
provenance.system_count
provenance.equation_count
provenance.residual_count
```

Uncertainty track records:

```text
typed_payload.uncertainties
```

Data-driven modeling track records:

```text
typed_payload.ml
```

The `typed_payload` is a Report seed. It carries computed statistics for the
official CSV path, integration metadata, policy results, and reviewable system
metadata. Future-track fields may carry deterministic uncertainty summaries and
data-driven modeling metrics/plot points:

```json
{
  "kind": "Report",
  "status": "ok",
  "result_format": "engres-v1",
  "vm_steps": [],
  "statistics": [],
  "integrations": [],
  "uncertainties": [],
  "ml": [],
  "policy_results": [],
  "systems": [],
  "solver_boundaries": [],
  "system_ir": []
}
```

## Tests

v0.9 includes:

```text
bytecode encode/decode unit test
VM scalar execution unit test
VM array seed unit test
VM TimeSeries object unit test
TimeSeries axis/statistics/integration compiler test
HeatRate sum lint smoke
PlotSpec JSON/SVG smoke
plot manifest smoke
output manifest smoke
file operation manifest smoke
run log artifact smoke
process result artifact smoke
test results artifact smoke
top-level workflow run smoke
official example run smoke
simple system run smoke
unit-aware print and explicit summary CSV export runtime test
write text/json overwrite hardening runtime test
copy/move/delete/mkdir file operation runtime test
log level and run_log runtime test
run command and process_results runtime test
test/assert/golden runtime test
```

## Deferred

Later versions will add:

```text
constant pool
typed IR serialization
function table
source maps
row-level table values
general table expression execution
general TimeSeries expression pages
numeric solver execution
adaptive or multi-equation solver execution
binary bytecode encoding
stable result schema validation
```
