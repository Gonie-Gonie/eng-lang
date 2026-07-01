# Weather API To Standard File

Source example: `examples/workflows/01_weather_api_to_standard_file/main.eng`

This workflow demonstrates a native API-to-artifact pattern:

```text
station map -> station selection -> http fixture/cache -> typed weather table -> coverage -> text artifacts
```

Run:

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file/main.eng --save-artifacts
```

What it proves:

- native `http get` with fixture, pinned SHA-256, retry, timeout, and cache key
- station-map CSV promotion and reviewable `select_first_row(...)`
- typed hourly-weather CSV promotion and TimeSeries coverage records
- generated `fetched_weather.json`, `standard_weather_file.txt`, and
  `weather_quality_summary.txt` through native `write text`
- `process_results.json` has `process_count = 0`

Expected review surfaces:

- `typed_payload.network_boundaries[]`
- `cache_manifest.json`
- `typed_payload.table_selections[]`
- `typed_payload.timeseries_coverage[]`
- `output_manifest.json` write-text artifact records

This is not a KMA, EPW, or building-energy adapter in core. Provider-specific
weather and standard-file adapters should layer above `eng.net`, `eng.cache`,
`eng.table`, `eng.timeseries`, and `eng.artifact`.
