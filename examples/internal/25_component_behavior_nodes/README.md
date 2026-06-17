# Internal 25 - Component Behavior Nodes

This fixture exercises valid component-local behavior node metadata for:

- `delay(out.T, 5 s)`
- `predictor(out.T)`
- `adapter(out.Q)`

Review/report/IDE artifacts must expose behavior nodes with source line,
signal, seed status, and solver integration limitations. This fixture does not
wire behavior nodes into numeric RHS/residual solving.
