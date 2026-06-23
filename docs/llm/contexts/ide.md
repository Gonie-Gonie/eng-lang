# IDE Context

Use this context for native IDE, LSP, and editor-adjacent work.

The native IDE should be described as an engineering review cockpit. The first
IDE story is not code editing and not solver debugging. It is inspection of:

- variables
- units and quantities
- schemas
- TimeSeries axes and values
- plots
- metrics and validations
- report/review artifacts
- side-effect artifacts
- provenance

Solver, component graph, and dependency panels are advanced inspection panels.
They should appear after TimeSeries/schema/unit/report panels in public docs.

The LSP and VS Code extension remain internal smoke/snapshot tooling unless a
status document declares a stable persistent editor-service contract.
