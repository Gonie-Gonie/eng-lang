# Schemas

Schemas define the typed boundary for CSV, JSON records, and table rows. They
are the main way to turn external data into reviewable EngLang tables.

## Schema Shape

```englang
schema Sensor {
    time: DateTime [iso8601]
    heat: HeatRate [kW]
    status: String
}
```

- Each column needs a name, a quantity/type, and a source unit or format marker.
- `DateTime [iso8601]` marks ISO-8601 timestamp columns.
- Optional/default field policy is supported where the config or promotion
  command defines that behavior.

## Promotion

- `promote csv file("data.csv") as Sensor` reads rows through a schema.
- `promote json records payload.records as Sensor` promotes JSON record arrays.
- Promotion records row diagnostics, source hashes, and table metadata for
  review/report artifacts.

## Related References

- [Syntax and grammar](syntax.md)
- [Artifact schema sources](../artifacts/schema_sources.md)
- [Report and review artifacts](../artifacts/report_review.md)
