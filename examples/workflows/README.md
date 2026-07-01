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

Current hybrid examples:

```text
01_weather_api_to_standard_file_hybrid
  API data -> typed schema -> quality/coverage review -> standard artifact.

02_external_simulation_surrogate_hybrid
  sample table -> explicit cases -> external runs -> typed results ->
  model-card/prediction artifacts -> DB side-effect manifest.

03_uncertain_sensor_report
  typed sensor data -> uncertainty metadata -> confidence-band report artifact.
```

Native workflow module contracts are documented separately:

```text
docs/workflows/weather_api_to_standard_file_native.md
  generic request/cache/table/coverage/artifact workflow contract.

docs/workflows/external_simulation_surrogate_native.md
  generic sample/case/process/model/prediction/DB workflow contract.
```

The hybrid examples are executable fixture pipelines. The native docs describe
the generic module contracts those fixtures are being reduced into. Domain
adapters such as weather APIs, EPW writers, and EnergyPlus-like tools should
remain layered above those generic modules.
