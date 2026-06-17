# Internal 25 - Component Behavior Nodes

This fixture exercises valid component-local behavior node metadata for:

- `temperature_signal = out.T`
- `delay(temperature_signal, 5 s)`
- `predictor(temperature_signal)`
- `adapter(out.Q)`

Review/report/IDE artifacts must expose behavior nodes with source line,
signal, inferred quantity-unit contract metadata including prior
component-local signal resolution, diagnostic channels, seed status, and solver
integration limitations. Runtime component solver artifacts must also state
that behavior graph nodes are present but not yet integrated into numeric
residual evaluation. This fixture does not wire behavior nodes into numeric
RHS/residual solving.
