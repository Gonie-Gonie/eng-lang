# Unit-Aware Print And CSV Summary Export

EngLang supports Python-like string interpolation for CLI/debug output and a
small explicit CSV export surface for scalar summary records.

Status: preview on `main`.

## Print

`print` writes to CLI/runtime stdout. It is intended for debugging and command
line feedback, not for reproducible artifact contracts.

```eng
print "Loaded {sensor.rows} rows from {args.input}"
print "Q mean = {mean(Q_coil, axis=Time): .2 kW}"
print "E total = {E_coil: .2 kWh}"
```

Interpolation fields are type-checked. A requested display unit must be
compatible with the expression quantity:

```text
{Q: .2 kW}   OK when Q is HeatRate/Power-compatible
{E: .2 kWh}  OK when E is Energy-compatible
{T: .1 degC} OK when T is AbsoluteTemperature-compatible
{L: .2 kW}   error: Length cannot be displayed as Power
```

Quantity values print with units by default. Tables and TimeSeries values print
as summaries by default; use scalar expressions such as `sensor.rows`,
`mean(Q_coil, axis=Time)`, or named scalar bindings for numeric formatting.

## CSV Summary Export

`export summary to csv` creates an explicit one-row summary record from the
fields in the block. It does not require a separate first-class `summary`
variable.

```eng
export summary to csv "summary.csv" {
    E_coil as kWh with ".2"
    peak_Q as kW with ".2"
    mean_Q as kW with ".2"
}
```

The path is written under `build/result` during `eng run`, even when ordinary
runtime artifacts remain in memory. For example, `"summary.csv"` writes
`build/result/summary.csv`.

CSV headers preserve display units, while cells contain formatted values:

```csv
E_coil [kWh],peak_Q [kW],mean_Q [kW]
0.37,6.91,5.11
```

Fields are type-checked with the same unit compatibility rule as `print`.
Supported field expressions include named scalar bindings, integration results,
and scalar statistics such as `mean(Q_coil, axis=Time)`.

## Boundary

`print` is for debug/CLI output. `export summary to csv`, `report`, and `show`
are artifact surfaces. Project-wide display-unit policy is intentionally not
specified here.
