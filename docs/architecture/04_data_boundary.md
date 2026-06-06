# Data Boundary and CSV Promote

v0.3-preview introduces the first typed data boundary seed.

The implemented preview path is:

```text
schema block
  -> schema symbol table
  -> promote csv expression
  -> CSV header read
  -> required column validation
  -> CSV source hash provenance
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

```eng
sensor = promote csv "data/sensor.csv" as SensorData
```

The path is resolved relative to the `.eng` source file.

Recorded promotion metadata:

```text
binding
schema name
source literal
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
```

## Artifacts

`review.json` includes:

```text
schemas
csv_promotions
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

## Current Limits

This is still a preview seed.

Deferred to later versions:

```text
- typed table object runtime
- DateTime value parsing
- row-level type conversion
- row-level constraint execution
- missing value interpolation
- TimeSeries construction
```

