# CLI Specification

The initial user-facing entry point is one executable: `eng.exe`.

## Commands

```text
eng.exe doctor
eng.exe new <project_name>
eng.exe check <file.eng> [--review]
eng.exe entries <file.eng>
eng.exe run <file.eng> [--entry <name>] [--open-report]
eng.exe build <file.eng> [--entry <name>] [--standalone] [--profile repro]
eng.exe view <result.engres>
eng.exe test <project_or_examples>
```

## `eng doctor`

Checks the local preview environment.

Current checks:

```text
Runtime
Standard library
Unit registry
Plot renderer
Report generator
Write permission
Example files
```

Success prints `Ready.` and returns exit code 0.

## `eng check <file.eng> [--review]`

Checks source and writes optional review metadata. It does not execute the entry point.

Current diagnostics:

```text
E-SYNTAX-DECL-001      := is not EngLang syntax
E-PUBLIC-ANNOTATION-001 schema columns require explicit quantity/unit annotations
E-DIM-ADD-001          Length + DimensionlessNumber is invalid
E-DIM-ADD-002          DimensionlessNumber + power quantity is invalid
E-DIM-ADD-003          AbsoluteTemperature + DimensionlessNumber is invalid
E-DIM-ADD-004          other physical quantity + DimensionlessNumber is invalid
E-RESERVED-KEYWORD-001 reserved keyword binding is invalid
W-QTY-AMBIG-001        ambiguous quantity warning
W-ENTRY-MAIN-001       non-main script entry warning
E-SCHEMA-PROMOTE-001   unknown schema in promote csv
E-SCHEMA-CSV-001       CSV source cannot be read
E-SCHEMA-CSV-002       CSV source missing required columns
E-SCHEMA-MISSING-001   missing policy references unknown column
E-ENTRY-NOT-FOUND-001  run/build entry point was not found
E-ENTRY-MULTIPLE-001   run/build entry point selection is ambiguous
W-STATS-SUM-001        HeatRate summed over Time should use integrate
```

`--review` writes:

```text
build/check/<source-stem>.review.json
```

Review JSON includes:

```text
review_schema_version
syntax_summary
quantity_completion_count
diagnostics
variable_table
warning_list
plot_manifest
entry_points
inferred_declarations
expected_types
hover_hints
type_info
unit_derivations
unit_conversion_table
axis_info
stats_info
integrations
schema_summary
schemas
csv_promotions
```

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng entries <file.eng>`

Lists script entry points discovered by the compiler.

Example:

```text
examples\04_plotting\main.eng:8: script main(args: Args) -> Report
```

This command is useful before running files with multiple script entries.

## `eng run <file.eng> [--entry <name>] [--open-report]`

Runs the selected entry through bytecode v1 and the native VM seed.

Default entry selection:

```text
1. If `--entry <name>` is passed, use that entry.
2. Otherwise, use `script main` when present.
3. Otherwise, use the only entry if the file has exactly one entry.
4. Otherwise, return an entry diagnostic.
```

Generated artifacts:

```text
build/
  <source-stem>.engbc
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

`--open-report` attempts to open the generated `report.html` with the OS default browser.

## `eng build <file.eng> [--entry <name>] --standalone --profile repro`

Creates a preview standalone package candidate:

```text
dist/
  <model>.exe
  <model>.engpkg
  <model>.lock
  <model>.review.html
```

The preview `.exe` remains a placeholder. The package records source hash, bytecode hash, and selected entry.

## `eng view <result.engres>`

Prints the result path, the sibling `report.html` and `report_spec.json` paths, and the plot manifest path when it exists.

The long-term result viewer will be connected to the typed `.engres` payload.

## `eng new <project_name>`

Creates a starter EngLang project:

```text
<project_name>/
  main.eng
  data/
    sensor.csv
```

## `eng test <project_or_examples>`

Runs official smoke checks:

```text
- official good examples check
- unit mismatch example produces errors
- ambiguous power example produces a warning
- HeatRate sum example produces W-STATS-SUM-001
- missing CSV column example produces errors
- missing entry example fails file run/build entry selection
- official plotting example produces report and PlotSpec artifacts
```
