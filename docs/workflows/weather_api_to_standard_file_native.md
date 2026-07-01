# Native Weather API To Standard File

The executable native example is:

```text
examples/workflows/01_weather_api_to_standard_file/main.eng
```

It uses `http get` with an offline fixture/cache record for the API boundary,
promotes station and weather CSV data to typed tables, checks TimeSeries
coverage, and writes standard text artifacts natively. No Python process is
called; saved runs should show `process_count = 0`.

Expected review surfaces:

- `typed_payload.network_boundaries[]`
- `cache_manifest.json`
- `typed_payload.table_selections[]`
- `typed_payload.timeseries_coverage[]`
- `output_manifest.json` write-text artifact records
