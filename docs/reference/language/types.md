# Types

EngLang types describe both value shape and engineering meaning. Use this page
as the quick lookup, then follow the linked reference page for the detailed
rules.

## Common Value Types

- `Bool`, `Int`, `Float`, `Number`, and `String` are scalar values.
- `Quantity [unit]` annotations attach engineering quantity and source unit to a
  binding, schema column, state, input, parameter, or output.
- `Optional[T]` is used for nullable config/schema fields where the surrounding
  command supports optional values.

## Data Boundary Types

- `CsvFile`, `JsonFile`, `TextFile`, `TomlFile`, `Path`, and `Url` describe
  typed file or network boundaries.
- `Table[T]` is a typed row table produced by promotion, sampling, transforms,
  case materialization, model prediction, or native DB-safe workflows.
- `TimeSeries[T]` is a time-indexed series used by statistics, plots,
  coverage checks, alignment, resampling, and simulation inputs.

## Workflow Artifact Types

- `Report`, `PlotSpec`, `ProcessResult`, `ModelCard`, `ModelArtifact`,
  `Prediction`, `SqliteConnection`, and `SqliteTable` are reviewable workflow
  artifact or boundary values.

## Related References

- [Syntax and grammar](syntax.md)
- [Dimensionless and unit policy](dimensionless.md)
- [TimeSeries statistics](timeseries.md)
- [Side-effect policy](side_effect_policy.md)
