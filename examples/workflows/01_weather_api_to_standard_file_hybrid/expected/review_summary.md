# Expected Review Summary

The saved run should show:

```text
schema StationMap with two fixture rows
selected station row, selected station_id, filters, and selection reason
schema WeatherHourly with two typed hourly fixture rows
generic DateTime coverage artifact with Gregorian-year expected count, missing interval, status, and max gap
read-only JSON source hash for sample_api_response.json
three explicit process boundaries
three expected process output contracts
one generated text artifact under outputs/standard_weather_file.txt
one generated quality summary artifact under outputs/weather_quality_summary.txt
report entries for selected station, coverage status, expected/missing counts, station rows, hourly rows, and max gap
```
