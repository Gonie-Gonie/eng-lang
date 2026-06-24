# Composite Workflow Examples

This directory contains workflow-shaped examples that combine current EngLang
primitives with explicit adapter boundaries. They are not first-user core
examples and they are not domain-specific product claims.

The examples demonstrate reusable patterns:

```text
typed input boundary
explicit external boundary
generated artifact and hash path
report/review artifact
fixture mode for deterministic smoke runs
```

Current examples:

```text
01_weather_api_to_standard_file_hybrid
  API data -> typed schema -> quality/coverage review -> standard artifact.

02_external_simulation_surrogate_hybrid
  sample table -> explicit cases -> external runs -> typed results ->
  model-card/prediction artifacts -> DB side-effect manifest.

03_uncertain_sensor_report
  typed sensor data -> uncertainty metadata -> confidence-band report artifact.
```

Planned modules such as `eng.net`, `eng.cache`, `eng.case`, `eng.db`, and
`eng.model` should grow from the repeated contracts in these examples. Domain
adapters such as weather APIs, EPW writers, and EnergyPlus-like tools should
remain layered above those generic modules.
