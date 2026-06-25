# Weather API To Standard File Hybrid

This workflow is not building-energy-specific.

It demonstrates a general pattern:

```text
API data -> typed schema -> quality check -> fallback/imputation -> standard artifact -> report
```

The current `main.eng` uses supported EngLang primitives for a deterministic
fixture run:

```text
typed station-map CSV promotion
typed hourly-weather CSV promotion
fixture JSON read with source hash provenance
explicit process boundaries represented by `run command`
generated standard-file text artifact
generated weather-quality summary artifact from CSV helper logic
report/review artifact generation
```

The Python tools in `tools/` show the adapter contract that future `eng.net`,
`eng.cache`, `eng.table`, `eng.timeseries`, and `eng.artifact` modules should
make native. They are deliberately generic and do not implement KMA or EPW as
core language behavior.

Target contract:

```text
args: year, region, station_map, output, optional api_key
promote station map
select station as a fixture-local binding until eng.table filtering exists
run external fetcher or use sample response
promote weather hourly data
coverage and missing-data report
run external standard-file writer
write output artifact
report station, coverage, missing count, and output path
```
