# Expected Output Manifest Summary

The saved run should include the standard artifact:

```text
outputs/fetched_weather.json
outputs/standard_weather_file.txt with `kind = standard_file`
outputs/weather_quality_summary.txt
```

The manifest should also include normal EngLang runtime artifacts when the
example is run with `--save-artifacts`. Generated output entries should include
hashes and validation sections.
