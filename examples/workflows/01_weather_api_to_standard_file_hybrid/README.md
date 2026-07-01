# Weather API To Standard File Hybrid

This hybrid workflow is not building-energy-specific. It is a native library
contract fixture for the generic `eng.net`, `eng.cache`, `eng.table`,
`eng.timeseries`, and `eng.artifact` target modules.

It demonstrates a general pattern:

```text
API data -> typed schema -> quality check -> fallback/imputation -> standard artifact -> report
```

The current `main.eng` uses supported EngLang primitives for a deterministic
fixture run:

```text
typed station-map CSV promotion
reviewable station row selection with `select_first_row(...)`
typed hourly-weather CSV promotion
fixture JSON read with source hash provenance
explicit process boundaries represented by `run command`
generated standard-file text artifact
generated weather-quality summary artifact from CSV helper logic
report/review artifact generation
```

The Python tools in `tools/` show a reviewable external adapter boundary that
native modules can replace later. They are deliberately generic fixture tools,
not the final architecture, and they do not implement KMA or EPW as core
language behavior.

## Native Module Replacement Map

| Current construct | Native module target |
|---|---|
| `read json api_fixture` | `eng.net.http_get` / `eng.net.download` |
| `select_first_row(stations, ...)` | `eng.table.filter` + `eng.table.require_one` |
| `check coverage weather.time` | `eng.timeseries.coverage` |
| `run command tools/fetch_weather_api.py` | `eng.net` + `eng.cache` |
| `run command tools/make_standard_weather_file.py` | `eng.artifact.standard_text_writer` |
| `run command tools/summarize_weather_quality.py` | `eng.quality` + `eng.report` |

Target contract:

```text
args: year, region, station_map, output, optional api_key
promote station map
select station from the promoted station map with `select_first_row(...)`
run external fetcher or use sample response
promote weather hourly data
generic DateTime coverage check with `check coverage weather.time` and missing-data report
run external standard-file writer with `artifact_kind = "standard_file"`
write output artifact with hash and validation metadata
report station, coverage status, expected/missing counts, and output path
```
