# Internal 25 - Component Behavior Nodes

This fixture exercises valid component-local behavior node metadata for:

- `temperature_signal = out.T`
- `delay(temperature_signal, 5 s)`
- `predictor(temperature_signal)`
- `adapter(out.Q)`

Review/report/IDE artifacts must expose behavior nodes with source line,
signal, inferred quantity-unit contract metadata including prior
component-local signal resolution, diagnostic channels, and explicit
declaration/execution status. This declaration-only fixture expects
`declared_not_executed`; it does not run numeric RHS/residual behavior
evaluation. The advanced solver 29, 30, and 31 fixtures cover native execution
for delay, Predictor, and external behavior nodes.
