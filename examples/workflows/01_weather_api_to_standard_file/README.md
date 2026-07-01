# Weather API To Standard File

This workflow demonstrates a native, domain-neutral API-to-artifact pattern:

```text
station table -> selected station -> args-driven HTTP fixture/cache boundary ->
response body artifact -> API JSON schema contract -> JSON records weather table ->
TimeSeries coverage -> generated text artifacts
```

The workflow uses:

```text
eng.net       http get args.api_url with selected station query, fixture, pinned SHA-256, retry, timeout, cache key
eng.cache     cache manifest and replayable fixture materialization from args-driven key parts
eng.config    read/promote JSON validation from the native HTTP response body
eng.table     station CSV promotion, JSON records table promotion, filter/require_one, and select_first_row
eng.timeseries coverage review for the hourly weather time axis
eng.artifact  write text artifacts with hashes and output manifest entries
```

Expected saved-run properties:

```text
process_results.json has process_count = 0
cache_manifest.json records the api_response network cache key from region/year args
result.engres records the resolved network query station value
result.engres typed_payload.config_promotions validates WeatherApiPayload from api_response.body
result.engres provenance.data_hashes records weather as source_format = json_records
output_manifest.json records fetched_weather.json, standard_weather_file.txt,
and weather_quality_summary.txt as native write_text artifacts
review.json records json_records table promotion, table transforms, network/cache boundary, and coverage data
```

Run:

```text
target\debug\eng.exe run examples\workflows\01_weather_api_to_standard_file\main.eng --save-artifacts
```
