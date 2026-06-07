# `eng run` Reference

`eng run` executes one file entry point through bytecode and the native VM seed.

## Basic Run

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng
```

Output:

```text
bytecode: build\main.engbc
result:   build\result\result.engres
review:   build\result\review.json
reportspec: build\result\report_spec.json
plot:     build\result\plots\timeseries.svg
plotspec: build\result\plots\plot_spec.json
manifest: build\result\plots\plot_manifest.json
report:   build\result\report.html
```

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

This attempts to open `build\result\report.html`.

## Args Flags

`struct Args` fields can be passed as `--<field> <value>` after the source path.
String defaults are used when the flag is omitted.

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

`review.json`, `report_spec.json`, and `result.engres` record `arg_values`
with `source = default` or `source = cli`.

## Simple System Example

```bat
target\debug\eng.exe run examples\official\02_simple_system\main.eng --entry main
```

This writes system/equation/residual metadata, system IR, solver_plan metadata,
and the runtime fixed-step ODE preview for the official simple thermal system
into:

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

After a successful run:

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
