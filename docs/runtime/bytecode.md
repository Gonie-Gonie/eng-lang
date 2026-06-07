# Bytecode VM and Result v1

v0.4-preview established the first executable EngLang runtime path. v0.5-preview adds TimeSeries/statistics metadata to that path. v0.6-preview adds PlotSpec/SVG/manifest output. v0.8-alpha adds system metadata, residuals, system IR dependencies, and an explicit unsolved solver boundary:

```text
.eng source
  -> compiler check
  -> entry selection
  -> .engbc bytecode v1
  -> native VM seed
  -> result.engres v1
  -> PlotSpec v1
  -> SVG + plot manifest
  -> review/report/system artifacts
```

The VM is intentionally small, but it is a real execution boundary: `eng run` writes bytecode, decodes it, executes the instruction stream, and writes the result from the VM execution record.

## Entry Selection

File run/build uses this rule:

```text
1. `--entry <name>` wins.
2. `script main(args: Args) -> Report` is the default when present.
3. A single non-main entry may run, but `check` emits W-ENTRY-MAIN-001.
4. Multiple entries without a default main require `--entry`.
5. No entries produces E-ENTRY-NOT-FOUND-001.
```

`eng check` does not require an entry. This keeps declaration-only files and error-message examples checkable.

## Bytecode v1

Header:

```text
ENGBYTECODE 1
format = engbc-v1
bytecode_version = 1
compiler_version = 0.6.0
source_hash = ...
source_bytes = ...
source_lines = ...
tokens = ...
ast_items = ...
typed_bindings = ...
schemas = ...
csv_promotions = ...
entry = script main
entry_args = args:Args
entry_return = Report
args_schema = Args metadata, when declared
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
0000|enter_entry|script|main
0001|load_table|sensor
0002|load_scalar|cp
0003|load_timeseries|Q_coil
0004|write_result|engres-v1
```

Current opcodes:

```text
enter_entry
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

Schema columns and v0.8 system variables are public boundary metadata, not runtime scalar values. The bytecode builder skips those bindings and emits runtime objects only for executable script bindings and promoted tables.
TimeSeries objects carry axis, quantity, and display-unit metadata.

## Result v1

`result.engres` uses JSON:

```text
format = engres-v1
result_format_version = 1
bytecode_version = 1
entry metadata
object_store summary
typed_payload
provenance
```

v0.6/v0.7 records:

```text
provenance.plot_spec_hash
provenance.report_spec_hash
```

v0.8 records:

```text
typed_payload.systems
typed_payload.solver_boundaries
typed_payload.system_ir
provenance.system_count
provenance.equation_count
provenance.residual_count
```

The `typed_payload` is a Report seed. It carries computed statistics for the
official CSV path, integration metadata, policy results, and reviewable system
metadata:

```json
{
  "kind": "Report",
  "status": "ok",
  "result_format": "engres-v1",
  "vm_steps": [],
  "statistics": [],
  "integrations": [],
  "policy_results": [],
  "systems": [],
  "solver_boundaries": [],
  "system_ir": []
}
```

## Tests

v0.4 includes:

```text
bytecode encode/decode unit test
VM scalar execution unit test
VM array seed unit test
VM TimeSeries object unit test
TimeSeries axis/statistics/integration compiler test
HeatRate sum lint smoke
PlotSpec JSON/SVG smoke
plot manifest smoke
entry not found run smoke
official example run smoke
simple system run smoke
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
binary bytecode encoding
stable result schema validation
```
