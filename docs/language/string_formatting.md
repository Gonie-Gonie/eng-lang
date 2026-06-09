# Unit-Aware Print And CSV Summary Export

EngLang supports Python-like string interpolation for CLI/debug output,
structured runtime messages, and a small explicit CSV export surface for scalar
summary records.

Status: preview on `main`.

## Print

`print` writes to CLI/runtime stdout. It is intended for debugging and command
line feedback, not for reproducible artifact contracts.

```eng partial
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

## Log

`log <level>` uses the same interpolation and unit compatibility rules as
`print`, but the message is structured for tools:

```eng partial
log debug "raw E = {E_coil: .3 kWh}"
log info "Q mean = {mean_Q: .2 kW}"
log warn "review high load case"
log error "operator acknowledgement required"
```

Supported levels are `debug`, `info`, `warn`, and `error`. `warn "..."` is not
a separate command; use `log warn "..."`.

During CLI runs, structured messages are printed with a level prefix such as
`[warn] review high load case`. Saved runs also write
`build/result/run_log.json` with level, rendered message, source line, and
source-order index.

## CSV Summary Export

`export summary to csv` creates an explicit one-row summary record from the
fields in the block. It does not require a separate first-class `summary`
variable.

```eng partial
export summary to csv "summary.csv" {
    E_coil as kWh with ".2"
    peak_Q as kW with ".2"
    mean_Q as kW with ".2"
}
```

The path is written under `build/result` during `eng run`, even when ordinary
runtime artifacts remain in memory. For example, `"summary.csv"` writes
`build/result/summary.csv`.

Existing identical output is accepted as an idempotent rerun. Replacing
different existing contents requires `with { overwrite = true }` attached to
the export block:

```eng partial
export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
}
with {
    overwrite = true
}
```

CSV headers preserve display units, while cells contain formatted values:

```csv
E_coil [kWh],peak_Q [kW],mean_Q [kW]
0.37,6.91,5.11
```

Fields are type-checked with the same unit compatibility rule as `print`.
Supported field expressions include named scalar bindings, integration results,
and scalar statistics such as `mean(Q_coil, axis=Time)`.

## Boundary

`print` is for direct debug/CLI output. `log <level>` is for structured runtime
messages. `export summary to csv`, `write text/json`, `report`, and `show` are
engineering artifact surfaces. Project-wide display-unit policy is
intentionally not specified here.
