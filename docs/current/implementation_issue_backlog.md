# Implementation Issue Backlog

This backlog captures post-1.0 implementation work that is outside the current
stable-core claim. Move these entries into GitHub Issues when issue-write
permissions are available.

## Formatter

Title: `formatter: add an EngLang source formatter for official examples`

Definition of Done:

- Add an `eng fmt` command or equivalent formatter entrypoint.
- Preserve comments and stable source semantics.
- Format `args`, `schema`, `system`, `report`, `where`, and `with` blocks
  consistently.
- Keep official examples formatter-clean.
- Add regression tests for formatter output.
- Document the formatter workflow in development docs.

## IDE Inspectors

Title: `ide: implement variable/unit/schema/TimeSeries inspectors`

Definition of Done:

- Variable table shows name, type, quantity, display unit, canonical unit,
  source expression, and source line.
- Unit table shows display/canonical conversion metadata.
- Schema table shows columns, constraints, missing policies, source hashes, and
  parse/conversion failures.
- TimeSeries inspector shows start/end, timestep or sample spacing, row count,
  missing count, source column, quantity/unit, canonical/display unit, and axis.
- Measured-vs-simulated workflow shows `weather_data`, `measured_data`,
  `sim.T_zone`, `rmse_T`, validation, time alignment, and two-series plot data.
- Add automated IDE smoke coverage.

Current coverage:

- IDE smoke covers schema/TimeSeries/metric/validation/time-alignment metadata
  for measured-vs-simulated and schema parse/conversion failure counts for a
  data-quality fixture.

Title: `ide: add side-effect artifact panels`

Current coverage:

- IDE `Effects` tab shows output-manifest artifacts, run-log messages,
  process results, and test results from the latest run.
- IDE smoke covers output manifest, run log, process results, and test results.

Definition of Done:

- Output manifest viewer lists generated artifacts and side-effect records.
- Run log viewer shows `print` and `log` records with level/source line.
- Process result viewer shows command, args, cwd, status, stdout/stderr, and
  duration.
- Test result viewer shows named tests, assertions, golden checks, and failures.
- Safe/normal/repro profile diagnostics are visible.
- Missing artifact files do not crash the IDE.

## Dynamic System I/O

Title: `system: support explicit TimeSeries input declarations`

Definition of Done:

- `input T_out: TimeSeries[Time] of AbsoluteTemperature [degC]` parses and
  checks.
- `simulate ... with { T_out = weather_data.T_out }` validates axis and
  quantity against the explicit input contract.
- Missing input, wrong quantity, wrong axis, invalid timestep, and unsupported
  solver diagnostics are covered.
- `sim.T_zone` remains materialized as a typed TimeSeries in result/report/IDE
  artifacts.
- Docs distinguish the current scalar input plus TimeSeries binding rule from
  the explicit TimeSeries input form.

## State-Space

Title: `state-space: implement actual trajectory generation or keep internal`

Definition of Done:

- Either keep `examples/internal/18_state_space_metadata` internal with clear
  docs and the current one-state TimeSeries-input trajectory preview, or
  implement a supported state-space workflow.
- Supported workflow requires runtime `StateVector`, `InputVector`, and
  `LinearOperator` objects, operator row/column checks, unit compatibility
  checks, discrete-time state update, state trajectory TimeSeries, plot/report
  output, IDE inspector support, and tests.
- No nonlinear/DAE/adaptive solver claim is made.

## Class/Domain Objects

Title: `class: close runtime object support before any stable claim`

Definition of Done:

- Runtime object representation exists for class literals and nested objects.
- Field access produces checked runtime values.
- Default fields, validation results, zero-argument metadata methods,
  copy-with behavior, and IDE object-summary inspection are covered by tests.
- Report/review artifacts include object summaries and validation results.
- IDE completion/hover shows fields, defaults, required fields, and units.
- Docs keep classes separate from systems/components and avoid solver claims.

## Component Graph

Title: `component: implement graph inspector without numeric solver claims`

Definition of Done:

- Component instances, ports, connect edges, domain labels, type arguments,
  medium/frame/axis metadata, and source spans are exposed as a graph artifact.
- Duplicate connection, unconnected port, incompatible domain, incompatible
  medium/frame/axis, and unsupported connect-pattern diagnostics are covered.
- IDE graph panel can navigate connections back to source.
- Report summarizes connection graph and limitations.
- Numeric component graph solving remains Planned.

Title: `assembly: harden generated equations and residual graph artifacts`

Definition of Done:

- Collect component instances, ports, connection sets, and generated connection
  equations.
- Record state/algebraic/input/output classification, equation count, unknown
  count, residual list, dependency graph, algebraic-loop seed, sparsity
  placeholder, and solver-plan placeholder.
- Report and IDE show generated equations and residual graph.
- Under/overdetermined cases produce diagnostics or limitation artifacts.
- No production multi-domain solver claim is made.
