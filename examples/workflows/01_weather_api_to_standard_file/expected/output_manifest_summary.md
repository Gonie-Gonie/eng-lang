# Expected Output Manifest Summary

The saved run should include the standard artifact:

```text
outputs/fetched_weather.json
outputs/standard_weather_file.txt with `kind = standard_file`
outputs/weather_quality_summary.txt
```

`weather_quality_summary.txt` should include the request method, resolved query
URL, boundary status, status code/class, response SHA-256, station, row count,
and coverage status.

The manifest should also include normal EngLang runtime artifacts when the
example is run with `--save-artifacts`. Generated output entries should include
hashes and validation sections.
