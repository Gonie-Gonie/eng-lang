# Bytecode VM and Result v1

v0.4-preview established the first executable EngLang runtime path. v0.5-preview adds TimeSeries/statistics metadata to that path. v0.6-preview adds PlotSpec/SVG/manifest output:

```text
.eng source
  -> compiler check
  -> entry selection
  -> .engbc bytecode v1
  -> native VM seed
  -> result.engres v1
  -> PlotSpec v1
  -> SVG + plot manifest
  -> review/report artifacts
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

Schema columns are public boundary metadata, not runtime scalar values. The bytecode builder skips schema column bindings and emits runtime objects only for executable script bindings and promoted tables.
TimeSeries objects carry axis, quantity, and display-unit metadata.

## Result v1

`result.engres` uses JSON in v0.4/v0.5:

```text
format = engres-v1
result_format_version = 1
bytecode_version = 1
entry metadata
object_store summary
typed_payload
provenance
```

v0.6 records:

```text
provenance.plot_spec_hash
```

The `typed_payload` is a Report seed. v0.5 adds lazy statistics and integration metadata:

```json
{
  "kind": "Report",
  "status": "ok",
  "result_format": "engres-v1",
  "vm_steps": [],
  "statistics": [],
  "integrations": []
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
```

## Deferred

Later versions will add:

```text
constant pool
typed IR serialization
function table
source maps
row-level table values
TimeSeries pages
PlotSpec payloads
binary bytecode encoding
stable result schema validation
```
