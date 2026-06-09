# `eng run` Reference

`eng run` executes one file's top-level workflow through bytecode and the native VM seed.
By default it keeps result, review, report, run-log, PlotSpec, SVG, output
manifest, and bytecode payloads as runtime objects and does not write ordinary
artifacts.
Explicit `export ... to csv`, `write text/json`, and constrained
`copy/move/delete` statements are user-requested artifacts and write or mutate
under `build\result`.

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
runlog:   456 bytes
plot:     3456 bytes
plotspec: 4567 bytes
plotmanifest: 678 bytes
outputs:  234 bytes
report:   7890 bytes
use --save-artifacts to write build\result files
```

Program `print` statements write before the runtime artifact summary:

```eng partial
print "Loaded {sensor.rows} rows from {args.input}"
log info "Q mean = {mean(Q_coil, axis=Time): .2 kW}"
print "E total = {E_coil: .2 kWh}"
```

Quantity formatting is unit-aware. Requested display units must be compatible
with the expression quantity.

Structured `log <level>` statements print with a level prefix and are also
recorded in `run_log.json` when artifacts are saved:

```eng partial
log debug "raw E = {E_coil: .3 kWh}"
log info "run complete"
log warn "review peak load"
log error "operator acknowledgement required"
```

## Save Artifacts

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts
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
    run_log.json
    output_manifest.json
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
An existing identical export is accepted as an idempotent rerun. Replacing
different existing contents requires `with { overwrite = true }` on the export.

## Explicit Write Outputs

`write text` and `write json` write small generated files under `build\result`.

```eng partial
write text "outputs/run_note.txt", "finished"
with {
    overwrite = true
}

write json "outputs/energy.json", E_coil
with {
    overwrite = true
}
```

The CLI reports write paths:

```text
write:    build\result\outputs\run_note.txt
write:    build\result\outputs\energy.json
```

Generated output files are listed in:

```text
build\result\output_manifest.json
```

## Explicit File Operations

`copy`, `move`, and `delete` provide a small filesystem mutation seed. The
current preview keeps generated-output mutations under `build\result`.

```eng partial
copy file("data/template.txt") to "ops/copied_note.txt"

move "ops/copied_note.txt" to "ops/archive/copied_note.txt"
with {
    confirm = true
    overwrite = true
}

write text "ops/scratch.txt", "temporary generated note"

delete "ops/scratch.txt"
with {
    confirm = true
}
```

The CLI reports touched file operation paths:

```text
fs:       build\result\ops\archive\copied_note.txt
fs:       build\result\ops\scratch.txt
```

`review.json` records `file_operations[]`. `output_manifest.json` records
entries such as `copy_file`, `move_file`, and `delete_file`.

## Structured Run Log

`log debug/info/warn/error` creates structured runtime message records. The CLI
prints the rendered messages with a `[level]` prefix, while saved runs write:

```text
build\result\run_log.json
```

The run-log artifact records:

```text
format = eng-run-log-v1
runtime_version
source_path
message_count
messages[]
  index
  level
  message
  line
```

Use `print` for quick direct output and `log <level>` when IDEs, CI scripts, or
review tools should read the message stream.

## Open Report

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --open-report
```

This writes artifacts and attempts to open `build\result\report.html`.

## Args Flags

`args { ... }` fields can be passed as `--<field> <value>` after the source
path. Defaults are used when the flag is omitted.

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --input data/sensor.csv
```

The official CSV example uses:

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
}

sensor = promote csv args.input as SensorData
```

The runtime result, report spec, and review objects record `arg_values` with
`source = default` or `source = cli`. With `--save-artifacts`, the same values
are written to `review.json`, `report_spec.json`, and `result.engres`.

Path helpers are available for CLI-bound path values:

```eng partial
input_exists = exists args.input
summary_path = join(args.output, "summary.csv")
input_name = stem(args.input)
```

`exists` is resolved relative to the source file when the path is relative. It
is recorded in `review.json` as `environment_dependencies` and in
`result.engres` / `report_spec.json` under `provenance.environment_dependencies`.

Primitive typed Args are normalized before they are recorded:

```text
String/Path/FilePath/CsvFile/JsonFile/TomlFile/TextFile/ReportFile/PlotFile/DirectoryPath  string/path value
Bool/Boolean         true/false, yes/no, on/off, 1/0
Int/Integer          signed whole number
Count/usize/u32/u64  non-negative whole number
Float/Number         finite numeric value
Duration             s, min, h normalized to seconds
```

Example:

```eng partial
args {
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
target\debug\eng.exe run examples\official\02_simple_system\main.eng
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

## Top-Level Workflow Example

Files run directly from top-level statements:

```eng
L = 1 m
print "L = {L: .2 m}"
```

`check` can inspect it and `run` executes it:

```bat
target\debug\eng.exe check examples\official\08_print_export_summary\main.eng
```

```bat
target\debug\eng.exe run examples\official\08_print_export_summary\main.eng
```

## Artifact Review

After a successful saved run:

```bat
type build\main.engbc
type build\result\result.engres
type build\result\review.json
type build\result\report_spec.json
type build\result\run_log.json
type build\result\output_manifest.json
type build\result\plots\plot_manifest.json
target\debug\eng.exe view build\result\result.engres
```

Inspect:

```text
.engbc
  ENGBYTECODE 1
  workflow
  objects
  instructions

result.engres
  format = engres-v1
  workflow
  arg_values
  object_store
  typed_payload
  provenance

review.json
  review_schema_version
  prints
  prints[].level
  variable_table
  unit_conversion_table
  arg_values
  system_summary
  system_ir
  system_ir.solver_plan
  schema_summary
  file_operations
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
