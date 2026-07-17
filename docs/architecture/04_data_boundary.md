# Data Boundary and CSV Promote

The data-boundary track materializes the official CSV example into runtime
table pages, TimeSeries values, computed statistics, and policy execution
status.

The implemented data-boundary path is:

```text
schema block
  -> schema symbol table
  -> promote csv expression
  -> CSV header read
  -> required column validation
  -> CSV source hash provenance
  -> runtime column pages
  -> source-unit to canonical-unit conversion metadata
  -> row-level policy checks
  -> review/report/result artifact metadata
```

## Schema Symbol Table

The compiler records:

```text
schema name
columns
column type
column source unit
index marker
constraints
missing policies
source line
```

Example:

```eng
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]

    constraints {
        time is monotonic
        m_dot >= 0 kg/s
    }

    missing {
        T_supply: interpolate max_gap=10 min
        m_dot: error
    }
}
```

## Promote CSV

The compiler recognizes:

```eng partial
sensor = promote csv "data/sensor.csv" as SensorData
```

The path is resolved relative to the `.eng` source file. CSV paths can also
come from root `args { ... }`:

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
}

sensor = promote csv args.input as SensorData
```

Run-time flags override defaults:

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts --input data/sensor.csv
```

Recorded promotion metadata:

```text
binding
binding span
expression span
promote keyword span
format keyword span
optional records keyword span
as keyword span
schema name
schema span
source literal
source span
source value after Args binding
resolved path
source hash
CSV headers
row count
missing columns
line
```

## Diagnostics

v0.3 adds:

```text
E-SCHEMA-PROMOTE-001
  CSV promotion references an unknown schema.

E-SCHEMA-CSV-001
  CSV source cannot be read.

E-SCHEMA-CSV-002
  CSV source is missing required schema column(s).

E-SCHEMA-MISSING-001
  Missing policy references an unknown schema column.

E-ARGS-CSV-001
  CSV promotion path references an Args field without a value.
```

Promotion syntax is parsed from lexer tokens, so ordinary spacing differences
do not change schema analysis. Schema-name diagnostics underline the exact name
after `as`; CSV/JSON/config source diagnostics underline the complete source
operand such as `args.input`, `payload.records`, or `file("data/input.csv")`.
When a source cannot be opened or parsed, the compiler reports that boundary
failure without also reporting required columns or fields as missing from an
empty synthetic payload.

## Database Targets

SQLite reads and writes use parser-owned `DbTableTargetDecl` metadata rather
than treating `<connection>.table("<name>")` as an opaque string. Semantic
analysis publishes the resolved connection path, table name, and exact source
ranges through `DbReadInfo` and `WriteInfo`. Connection and schema diagnostics
therefore select the failing connection or table occurrence even when the same
text appears earlier on the line.

The native runtime consumes these structured read/write targets directly when
materializing tables or applying writes. It does not reconstruct a DB target by
splitting normalized expression text. The legacy expression helper remains a
compatibility parser for external callers, not the canonical compiler-to-runtime
boundary.

## Artifacts

`review.json` includes:

```text
schemas
csv_promotions
arg_values
```

`report.html` includes:

```text
Schemas
CSV Promotions
```

`result.engres` includes provenance:

```text
schema_count
csv_promotion_count
data_hashes
```

Runtime table column objects also include:

```text
unit
canonical_unit
values
canonical_values
conversion_failures
```

`values` preserve the CSV source/display unit used by the current runtime
calculation path. `canonical_values` record each numeric cell converted into
the quantity's canonical unit when a registered conversion rule exists.
Unsupported source units are recorded in `conversion_failures` with row,
column, value, source unit, target unit, and message fields.

`result.engres` and `report_spec.json` include runtime policy results:

```text
kind = constraint | missing
status = recorded | validated | executed
checked_rows
violation_count
violations
```

## Current Limits

Implemented for the official CSV path:

```text
- RuntimeTable column values
- DateTime index parsing to seconds offsets
- numeric quantity column parsing
- per-cell source-unit to canonical-unit metadata for registered units
- per-cell conversion failure diagnostics for unsupported source units
- time is monotonic checks
- between checks
- numeric bound checks such as m_dot >= 0 and m_dot <= 0.25
- missing error policy checks
- numeric missing value interpolation with surrounding values
```

Deferred to later versions:

```text
- broad row expression execution outside supported monotonic, between, and bound policies
```
