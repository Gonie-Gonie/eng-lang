# Weather API To Standard File

Source example: `examples/workflows/01_weather_api_to_standard_file/main.eng`

This workflow demonstrates a native API-to-artifact pattern:

```text
station map -> station selection -> args-driven pinned HTTP response/cache -> native response metadata -> typed weather table -> coverage -> standard text and quality artifacts
```

Run:

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file/main.eng --save-artifacts
```

What it proves:

- native `http get args.api_url` with resolved selected-station query, pinned
  offline response input, SHA-256, retry, timeout, cache key, response status,
  response hash, and resolved query URL fields
- station-map CSV promotion and reviewable `filter`/`require_one` row selection
- typed API JSON-record table promotion and TimeSeries coverage records
- generated `fetched_weather.json` and `weather_quality_summary.txt` through
  native `write text`, plus `standard_weather_file.txt` through native
  `write standard_text`
- typed `args.output` routing for the fetched payload, standard weather file,
  and weather quality summary
- workflow bindings can use `api_response.method`, `api_response.status`,
  `api_response.status_code`, `api_response.status_class`,
  `api_response.response_hash`, and `api_response.url_with_query`
- `process_results.json` has `process_count = 0`

Expected review surfaces:

- `typed_payload.network_boundaries[]`
- `cache_manifest.json`
- `typed_payload.table_transforms[]`
- `typed_payload.timeseries_coverage[]`
- `static_run_plan.json`, `run_plan.json`, and `run_lock.json`
- `output_manifest.json` write-text and standard-file artifact records

This is not a KMA, EPW, or building-energy adapter in core. Provider-specific
weather and standard-file adapters should layer above `eng.net`, `eng.cache`,
`eng.table`, `eng.timeseries`, `eng.workflow`, and `eng.artifact`.
