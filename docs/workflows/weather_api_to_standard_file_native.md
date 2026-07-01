# Native Weather API To Standard File

Purpose: document the native generic module pattern for turning an API or
fixture payload into a typed standard file without making any weather provider
or file format part of the core language.

Source contract: this is the native module contract for the hybrid workflow at
`examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng`.

## Domain-Neutral Pattern

```text
request or fixture -> cache record -> typed table -> coverage/quality review -> standard file artifact
```

The domain adapter supplies endpoint details, provider credentials, and the
standard-file writer. EngLang owns typed inputs, explicit side effects,
cache/network review records, table diagnostics, TimeSeries coverage, output
manifests, and report/review artifacts.

## Source Code

```eng unchecked
schema StationMap {
    region: String
    station_id: String
}

schema WeatherHourly {
    time: DateTime index
    dry_bulb: AbsoluteTemperature [degC]
}

stations = promote csv file("data/station_map_sample.csv") as StationMap
station = select_first_row(stations, return_column="station_id", region="demo", start=date(2024, 1, 1), end=date(2024, 12, 31))

api_response = read json file("data/sample_api_response.json")
weather = promote csv file("data/sample_weather_hourly.csv") as WeatherHourly
coverage = check coverage weather.time for year 2024

writer = run command "python"
with {
    args = ["tools/make_standard_weather_file.py"]
    expected_outputs = ["outputs/standard_weather_file.txt"]
    artifact_kind = "standard_file"
}
```

## Run Command

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng --save-artifacts
```

For the native workflow module gate:

```bat
.\dev.bat workflows-test
```

## Expected Artifacts

- `review.json`
- `output_manifest.json`
- `run_log.json`
- `run_plan.json`
- `run_lock.json`
- `process_results.json`
- `cache_manifest.json`
- standard-file output record in `output_manifest.json`
- table diagnostics and TimeSeries coverage records in `result.engres`

## Review Checklist

- API or fixture source is explicit and hashable.
- Cache/network records show request status, cache key, and cache hit/miss
  status when used.
- Promoted station and hourly tables have schema diagnostics.
- The selected station row is reviewable.
- Coverage records show expected count, actual count, missing count, max gap,
  and leap-year policy.
- The standard-file artifact has path, kind, hash, and validation status.

## Failure Modes

- missing or malformed API/fixture payload
- unresolved station row or duplicate station match
- schema mismatch in promoted table
- TimeSeries coverage gaps
- standard-file writer exits nonzero or misses expected output
- cache hash mismatch under reproducible profile

## Extension Points For Domain Adapters

- Replace the fixture reader with a provider-specific HTTP adapter.
- Replace the generic standard-file writer with an EPW, TMY, or provider export
  adapter.
- Keep adapter outputs as typed tables and artifact records so the adapter can
  be swapped without changing core language syntax.
