# Expected Review Summary

The saved run should show:

```text
schema StationMap with two station rows
filter/require_one station transform, resolved selected station_id from station.station_id, predicates, and row diagnostics
schema WeatherApiRecord with two typed hourly records promoted from api_payload.records
WeatherApiPayload contract validated from read json api_response.body
generic DateTime coverage artifact with Gregorian-year expected count, missing interval, status, and max gap
native http get boundary for api_response with args.api_url, resolved station query, pinned offline response, SHA-256, retry, timeout, and cache key
network cache entry owned by network_request/api_response with region/year key parts
review/provenance entries showing weather source_format = json_records
fetched_weather.json materialized from api_response.body
process_results.json with process_count = 0
generated text artifacts under outputs/fetched_weather.json, outputs/standard_weather_file.txt, and outputs/weather_quality_summary.txt
report entries for selected station, coverage status, expected/missing counts, station rows, hourly rows, and max gap
```
