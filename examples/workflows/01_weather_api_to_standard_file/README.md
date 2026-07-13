# Weather API To Standard File

This workflow demonstrates a native, domain-neutral API-to-artifact pattern:

```text
station table -> selected station -> args-driven HTTP boundary with args.pinned_response_file/cache replay ->
native HTTP response object metadata -> response body artifact -> API JSON
schema contract -> JSON records weather table -> TimeSeries coverage ->
native standard_text artifact and quality text artifact
```

The workflow uses:

```text
eng.net       http get args.api_url with selected station query, args.pinned_response_file, SHA-256, retry, timeout, cache key, response source/status-code/hash/query URL fields
eng.cache     cache records and replayable response materialization from args-driven key parts
eng.config    direct JSON promotion validation from the native HTTP response body
eng.table     station CSV promotion, JSON records table promotion, and filter/require_one row selection
eng.timeseries coverage review for the hourly weather time axis
eng.artifact  write standard_text table artifact plus generated text artifacts with hashes and output manifest entries
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
cache_manifest.json records the api_response network cache key from region/year args and the pinned response materialization path
result.engres records the resolved network query station value from station.station_id
weather_quality_summary.txt records the native response source, status code, status class, response hash, and query URL
result.engres typed_payload.config_promotions validates WeatherApiPayload from api_response.body
result.engres provenance.data_hashes records weather as source_format = json_records
output_manifest.json records fetched_weather.json and weather_quality_summary.txt
as write_text artifacts, and standard_weather_file.txt as a standard_file artifact
review.json records json_records table promotion, table transforms, network/cache boundary, response-source metadata bindings, and coverage data
typed args.output controls the fetched payload, standard weather file, and quality summary output paths
```

This checked workflow keeps `offline_response` enabled so CI and local smoke
runs do not depend on a live weather service. The same native `http get` path
also supports live HTTP(S) execution and cached replay when the pinned response
option is removed and a reachable URL is supplied.

Run:

```text
target\debug\eng.exe run examples\workflows\01_weather_api_to_standard_file\main.eng --save-artifacts
```
