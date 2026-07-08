# 04 Schema And CSV Promote

## Goal

Load CSV data through a typed schema so columns, units, indexes, constraints,
and missing-data policy become part of the workflow.

## What You Will Build

The official CSV example promotes sensor data:

```eng partial
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]

    constraints {
        time is monotonic
        m_dot >= 0 kg/s
    }
}

sensor = promote csv args.input as SensorData
```

## Source File

Use examples/official/01_csv_plot/main.eng.

## Execute The Program

```bat
eng.exe run examples/official/01_csv_plot/main.eng --save-artifacts
```

## Expected Artifacts

The run should produce schema promotion evidence, derived TimeSeries values, a
plot, a report, and review JSON.

## Explanation

promote csv is the boundary between untyped table data and EngLang's typed
engineering model. After promotion, downstream calculations can rely on column
quantity kinds, units, and index semantics.

## Common Mistakes

- Letting CSV headers drift from schema field names.
- Omitting units from schema fields that need physical meaning.
- Treating missing data as a later plotting problem instead of a schema
  boundary decision.

## What To Inspect

Open the review artifact and check schema rows, missing-data policy results,
constraint status, and converted unit metadata.
