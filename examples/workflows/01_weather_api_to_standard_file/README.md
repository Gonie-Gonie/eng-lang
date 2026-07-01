# Weather API To Standard File

This workflow demonstrates a native, domain-neutral API-to-artifact pattern:

```text
station table -> selected station -> native HTTP fixture/cache boundary ->
response body artifact -> typed weather table -> TimeSeries coverage ->
generated text artifacts
```

The workflow uses:

```text
eng.net       http get with fixture, pinned SHA-256, retry, timeout, cache key
eng.cache     cache manifest and replayable fixture materialization
eng.table     CSV promotion plus filter/require_one station selection
eng.timeseries coverage review for the hourly weather time axis
eng.artifact  write text artifacts with hashes and output manifest entries
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
cache_manifest.json records the api_response network cache key
output_manifest.json records fetched_weather.json, standard_weather_file.txt,
and weather_quality_summary.txt as native write_text artifacts
review.json records table transforms, network/cache boundary, and coverage data
```

Run:

```text
target\debug\eng.exe run examples\workflows\01_weather_api_to_standard_file\main.eng --save-artifacts
```
