# Composite Workflow Examples

This directory contains executable native workflow examples. They combine
generic EngLang workflow modules rather than domain-specific adapters.

Current examples:

```text
01_weather_api_to_standard_file
  native network/cache response object -> typed weather table -> coverage review ->
  generated standard weather and request-metadata artifacts.

02_native_surrogate_case_workflow
  native LHS input samples -> native derived result table -> native case
  template apply -> table-based regression -> native prediction table -> SQLite writes.

03_uncertain_sensor_report
  typed sensor data -> native coverage/summary bindings -> generated
  CSV/text artifacts -> uncertainty metadata -> confidence-band report artifact.
```

All three workflows stay inside EngLang native workflow modules. They do not
call language-external interpreters, interactive script artifacts, or
command-process workflow steps; `dev.bat workflows-test` enforces that contract
across source files, public workflow docs, run graph artifacts, and
`process_results.json`. Saved runs still write `process_results.json`, but its
`process_count` is expected to be zero for these native workflows.

Run them from the repository root:

```text
target\debug\eng.exe run examples\workflows\01_weather_api_to_standard_file\main.eng --save-artifacts
target\debug\eng.exe run examples\workflows\02_native_surrogate_case_workflow\main.eng --save-artifacts
target\debug\eng.exe run examples\workflows\03_uncertain_sensor_report\main.eng --save-artifacts
```

Domain adapters such as KMA, EPW, EnergyPlus, CFD, FEM, or vendor DB/ML tools
should remain thin layers above these generic modules.
