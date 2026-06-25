# Expected Review Summary

The saved run should show:

```text
schema StationMap with two fixture rows
schema WeatherHourly with two typed hourly fixture rows
read-only JSON source hash for sample_api_response.json
three explicit process boundaries
three expected process output contracts
one generated text artifact under outputs/standard_weather_file.txt
one generated quality summary artifact under outputs/weather_quality_summary.txt
report entries for selected station, station rows, hourly rows, missing count,
and max gap
```
