# Bytecode VM and Result v1

The current package has an executable EngLang runtime path. The compact
bytecode VM loads declared scalar/table/TimeSeries/array objects and establishes
the result boundary. Native runtime materialization then evaluates the supported
table, TimeSeries, workflow-module, uncertainty, model, and numeric solver
operations before report artifacts are serialized:

```text
  .eng source
  -> compiler check
  -> top-level workflow metadata
  -> .engbc bytecode v1
  -> native VM execution
  -> result.engres v1
  -> PlotSpec v1
  -> SVG + plot manifest + run log + process results + test results + output manifest
  -> review/report/system artifacts
```

The VM is intentionally small, but it is a real execution boundary: `eng run`
builds bytecode, decodes it, executes the instruction stream, and uses the VM
execution record as the object-store base. The native runtime then materializes
numeric values and workflow artifacts against compiler-owned semantic data.
`--save-artifacts` writes both the bytecode record and the fully
materialized result to disk. Numeric support in the runtime must not be confused
with numeric opcodes in bytecode v1; those opcodes do not exist yet.

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

Schema columns and system variables are public boundary metadata, not automatic
runtime scalar objects. The bytecode builder skips those declarations and emits
objects for executable top-level bindings and promoted tables. Runtime
materialization can later add concrete table, TimeSeries, case, model, and
simulation shapes to the VM object-store summary. TimeSeries objects carry axis,
quantity, and display-unit metadata.

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
typed_payload.component_solutions
provenance.system_count
provenance.equation_count
provenance.residual_count
```

The compiler-owned `system_ir` starts with an
`unsolved`/`metadata_only` plan. Runtime simulation
materializes one-state thermal, multi-state source-equation ODE, and typed-block
state-space results. Targeted source `solve` requests materialize
dense-linear, fixed-point, Newton, implicit-Euler DAE, dynamic-component,
behavior-node, and constrained component/domain residual results. The result
records preserve method, trajectory or solved values, convergence evidence,
step diagnostics, residuals, and explicit failure artifacts.

Uncertainty track records:

```text
typed_payload.uncertainties
typed_payload.timeseries_uncertainty_calculations
```

Data-driven modeling track records:

```text
typed_payload.ml
typed_payload.model_specs
typed_payload.model_cards
typed_payload.prediction_manifests
```

The `typed_payload` is the runtime report payload. It carries computed
statistics and integrations, table/sample/case/model/DB records, uncertainty
calculations, quality and policy results, and reviewable system/component solver
results. The abbreviated shape below shows stable top-level groups rather than
every current workflow field:

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
VM array object unit test
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
multi-state source-equation and state-space runtime tests
dense-linear, fixed-point, Newton, DAE, and dynamic-component runtime tests
native workflow 01/02/03 artifact and zero-process contract tests
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
typed workflow and numeric solver opcodes in the bytecode instruction stream
serialized solver IR independent of compiler semantic data
event-aware and broad production component/multi-domain solving
binary bytecode encoding
```
