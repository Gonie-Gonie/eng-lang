# Data Boundary and CSV Promote

v0.3-preview introduced the first typed data boundary seed. The current v1.0
hardening path materializes the official CSV example into runtime table pages,
TimeSeries values, computed statistics, and policy execution status.

The implemented preview path is:

```text
schema block
  -> schema symbol table
  -> promote csv expression
  -> CSV header read
  -> required column validation
  -> CSV source hash provenance
  -> runtime column pages
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
come from `struct Args`:

```eng partial
struct Args {
    input: String = "data/sensor.csv"
}

script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData
}
```

Run-time flags override defaults:

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main --input data/sensor.csv
```

Recorded promotion metadata:

```text
binding
schema name
source literal
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
- time is monotonic checks
- between checks
- numeric bound checks such as m_dot >= 0 and m_dot <= 0.25
- missing error policy checks
- numeric missing value interpolation with surrounding values
```

Deferred to later versions:

```text
- broad row expression execution outside supported monotonic, between, and bound policies
- per-cell unit conversion diagnostics once conversion exists
```
