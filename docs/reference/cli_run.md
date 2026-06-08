# `eng run` Reference

`eng run` executes one file entry point through bytecode and the native VM seed.
By default it keeps result, review, report, PlotSpec, SVG, and bytecode payloads
as runtime objects and does not write files. Explicit `export ... to csv`
statements are user-requested artifacts and write under `build\result`.

## Basic Run

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng
```

Output:

```text
run: ok
artifacts: in memory
result:   1234 bytes
review:   5678 bytes
reportspec: 2345 bytes
plot:     3456 bytes
plotspec: 4567 bytes
manifest: 678 bytes
report:   7890 bytes
use --save-artifacts to write build\result files
```

Program `print` statements write before the runtime artifact summary:

```eng partial
print "Loaded {sensor.rows} rows from {args.input}"
print "Q mean = {mean(Q_coil, axis=Time): .2 kW}"
print "E total = {E_coil: .2 kWh}"
```

Quantity formatting is unit-aware. Requested display units must be compatible
with the expression quantity.

## Save Artifacts

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main --save-artifacts
```

This writes the current artifact set:

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    report_spec.json
    plots/
      plot_spec.json
      plot_manifest.json
      timeseries.svg
```

## Explicit CSV Summary Export

`export summary to csv` writes a one-row scalar summary record under
`build\result`, even when ordinary runtime artifacts stay in memory.

```eng partial
mean_Q = mean(Q_coil, axis=Time)
peak_Q = max(Q_coil, axis=Time)
E_coil = integrate(Q_coil, over=Time)

export summary to csv "summary.csv" {
    E_coil as kWh with ".2"
    peak_Q as kW with ".2"
    mean_Q as kW with ".2"
}
```

The CLI reports the export path:

```text
export:   build\result\summary.csv
```

CSV headers include display units and cells contain formatted scalar values.

## List Entries

```bat
target\debug\eng.exe entries examples\official\01_csv_plot\main.eng
```

Output:

```text
examples\official\01_csv_plot\main.eng:25: script main(args: Args) -> Report
```

## Select an Entry

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main
```

Default rule:

```text
1. `--entry <name>` wins.
2. `script main` is the default when present.
3. A single non-main entry can run.
4. Multiple non-main entries require `--entry`.
5. No entry fails with E-ENTRY-NOT-FOUND-001.
```

## Open Report

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --open-report
```

This writes artifacts and attempts to open `build\result\report.html`.

## Args Flags

`struct Args` fields can be passed as `--<field> <value>` after the source path.
Defaults are used when the flag is omitted.

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main --input data/sensor.csv
```

The official CSV example uses:

```eng partial
struct Args {
    input: String = "data/sensor.csv"
}

script main(args: Args) -> Report {
    sensor = promote csv args.input as SensorData
}
```

The runtime result, report spec, and review objects record `arg_values` with
`source = default` or `source = cli`. With `--save-artifacts`, the same values
are written to `review.json`, `report_spec.json`, and `result.engres`.

Primitive typed Args are normalized before they are recorded:

```text
Bool/Boolean         true/false, yes/no, on/off, 1/0
Int/Integer          signed whole number
Count/usize/u32/u64  non-negative whole number
Float/Number         finite numeric value
Duration             s, min, h normalized to seconds
```

Example:

```eng partial
struct Args {
    enabled: Bool = false
    count: Count = 3
    gain: Float = 1.0
    window: Duration = 5 min
}
```

```bat
target\debug\eng.exe run model.eng --enabled yes --count 12 --gain 1.25 --window "10 min"
```

The recorded values are `enabled=true`, `count=12`, `gain=1.25`, and
`window=600 s`. Invalid typed values produce `E-ARGS-TYPE-001`.

## Simple System Example

```bat
target\debug\eng.exe run examples\official\02_simple_system\main.eng --entry main
```

This produces system/equation/residual metadata, system IR, solver_plan
metadata, and the runtime fixed-step ODE preview for the official simple
thermal system. Add `--save-artifacts` to write them into:

```text
build\result\review.json
build\result\report_spec.json
build\result\result.engres
build\result\report.html
```

## Missing Entry Example

`examples\05_error_messages\missing_entry.eng` is intentionally declaration-only:

```eng
L = 1 m
```

`check` can inspect it:

```bat
target\debug\eng.exe check examples\05_error_messages\missing_entry.eng
```

`run` fails because file execution requires an entry point:

```bat
target\debug\eng.exe run examples\05_error_messages\missing_entry.eng
```

Expected diagnostic:

```text
E-ENTRY-NOT-FOUND-001
```

## Artifact Review

After a successful saved run:

```bat
type build\main.engbc
type build\result\result.engres
type build\result\review.json
type build\result\report_spec.json
type build\result\plots\plot_manifest.json
target\debug\eng.exe view build\result\result.engres
```

Inspect:

```text
.engbc
  ENGBYTECODE 1
  entry
  objects
  instructions

result.engres
  format = engres-v1
  entry
  arg_values
  object_store
  typed_payload
  provenance

review.json
  review_schema_version
  variable_table
  unit_conversion_table
  arg_values
  system_summary
  system_ir
  system_ir.solver_plan
  schema_summary
  warning_list

report_spec.json
  format = eng-report-spec-v1
  variable_table
  inferred_declaration_table
  unit_conversion_table
  arg_values
  system_summary
  system_ir
  system_ir.solver_plan
  plot_manifest
```
