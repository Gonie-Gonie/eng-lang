# Weather API To Standard File

Source example: examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng

This workflow demonstrates a general adapter pattern:

```text
API data -> typed schema -> quality check -> fallback/imputation -> standard artifact -> report
```

## Run

```bat
eng.exe run examples/workflows/01_weather_api_to_standard_file_hybrid/main.eng --save-artifacts
```

## What It Proves

- station-map CSV promotion
- typed hourly-weather CSV promotion
- fixture JSON read with source hash provenance
- explicit process boundaries for fetch and standard-file writing
- generated standard-file and quality-summary artifacts
- report/review artifact generation

## What It Does Not Claim

This is not KMA, EPW, or building-energy behavior in the core language.
Weather-specific adapters should live above generic data, cache, process,
artifact, and report modules.

## Review Points

Inspect selected station evidence, source hashes, coverage status,
quality-summary output, standard-file path, and review/report artifacts.
