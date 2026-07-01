# Expected Review Summary

The saved run should show:

```text
schema StationMap with two fixture rows
filter/require_one/select_first_row station transform, resolved selected station_id, predicates, and row diagnostics
schema WeatherHourly with two typed hourly fixture rows
generic DateTime coverage artifact with Gregorian-year expected count, missing interval, status, and max gap
native http get boundary for api_response with args.api_url, resolved station query, fixture, pinned SHA-256, retry, timeout, and cache key
network cache entry owned by network_request/api_response with region/year key parts
fetched_weather.json materialized from api_response.body
process_results.json with process_count = 0
generated text artifacts under outputs/fetched_weather.json, outputs/standard_weather_file.txt, and outputs/weather_quality_summary.txt
report entries for selected station, coverage status, expected/missing counts, station rows, hourly rows, and max gap
```
