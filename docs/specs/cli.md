# CLI Specification

The core user-facing CLI is `eng.exe`. Portable tester IDE releases also ship
`eng-ide.exe` as a native GUI companion.

## Commands

```text
eng.exe doctor
eng.exe new <project_name>
eng.exe check <file.eng> [--review]
eng.exe ide-check <file.eng>
eng.exe jit-plan <file.eng> [--backend <name>]
eng.exe jit-bench <file.eng> [--iterations N] [--backend <name>] [--<arg> <value>...]
eng.exe run <file.eng> [--open-report] [--save-artifacts] [--<arg> <value>...]
eng.exe build <file.eng> [--standalone] [--profile repro]
eng.exe view <result.engres>
eng.exe test <project_or_examples>
eng-ide.exe
eng-ide.exe --smoke
eng-lsp.exe --smoke
eng-lsp.exe --snapshot <file.eng>
eng-lsp.exe --snapshot-check <file.eng>
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

Checks source and writes optional review metadata. It does not execute the
top-level workflow.

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
E-SCHEMA-PROMOTE-001   unknown schema in promote csv
E-SCHEMA-CSV-001       CSV source cannot be read
E-SCHEMA-CSV-002       CSV source missing required columns
E-SCHEMA-MISSING-001   missing policy references unknown column
E-ARGS-UNKNOWN-001     CLI Args flag does not match `args { ... }`
E-ARGS-REQUIRED-001    required Args field was not provided for run
E-ARGS-TYPE-001        Args value cannot be converted to the declared type
E-ARGS-CSV-001         CSV promotion references an Args field without a value
E-SCRIPT-001           `script` blocks are not supported as execution roots
W-STATS-SUM-001        HeatRate summed over Time should use integrate
E-EQ-BOOL-001          physical equation used == instead of eq
E-EQ-UNIT-001          physical equation dimensions do not match
E-UNC-SOURCE-001      missing or unknown uncertainty source reference
E-UNC-SOURCE-002      referenced binding is not an uncertainty source
E-UNC-ARGS-001        missing or malformed required uncertainty argument
E-UNC-ARGS-002        invalid numeric/range/count/transform uncertainty argument
E-UNC-ARGS-003        unsupported uncertainty option
E-DOMAIN-CONTRACT-001  domain has no across variable
E-DOMAIN-CONTRACT-002  domain has no through variable
E-DOMAIN-CONTRACT-003  domain has no conservation contract
E-DOMAIN-VAR-001       domain variable uses an unknown quantity kind
E-PORT-DOMAIN-001      component port references an unknown domain
E-PORT-DOMAIN-002      generic domain reference has wrong argument count
E-CONNECT-ENDPOINT-001 connection endpoint is not Component.port
E-CONNECT-PORT-001     connection endpoint does not resolve to a port
E-CONNECT-DOMAIN-001   connected ports have incompatible domains
E-CONNECT-MEDIUM-001   connected generic ports have incompatible Medium arguments
E-CONNECT-FRAME-001    connected generic ports have incompatible Frame arguments
E-CONNECT-AXIS-001     connected generic ports have incompatible Axis arguments
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
workflow
args_summary
arg_values
inferred_declarations
expected_types
hover_hints
type_info
unit_derivations
unit_conversion_table
axis_info
stats_info
integrations
system_summary
domain_summary
component_summary
connection_summary
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

## `eng ide-check <file.eng>`

Prints the same review JSON used by `eng check --review` to stdout instead of
writing it under `build/check`.

This command is intended for IDE tools and extensions that need diagnostics,
hover hints, type information, symbols, Args metadata, schema metadata, and
completion counts without managing generated review files.

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng jit-plan <file.eng>`

Prints experimental `eng-kernel-plan-v1` JSON for runtime optimization track hot-kernel planning.
This command does not compile native code and does not change runtime
execution. Its current backend is `interpreter-fallback`.

Supported backend requests are `auto`, `interpreter-fallback`, and
`native-preview`. `native-preview` records a request but still selects
`interpreter-fallback` with `backend_selection.status = not_available`.

Each candidate includes source, reason, lowering status, operation list, and a
coarse planning estimate:

```text
estimated_rows
input_count
output_count
operation_count
scan_count
complexity
notes
```

These estimates are for inspection and benchmark selection only. They are not
measured performance data.

Current candidate kinds:

```text
timeseries_arithmetic
timeseries_integrate
statistics_fusion
system_residual
```

Example:

```bat
eng.exe jit-plan examples\official\01_csv_plot\main.eng
```

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng jit-bench <file.eng>`

Runs an experimental `eng-jit-bench-v1` benchmark harness for runtime optimization track planning.
The harness measures the current interpreter/runtime path for a small number of
iterations and includes the current `eng-kernel-plan-v1` metadata in the same
JSON output.

Current behavior:

```text
- default iterations: 3
- allowed iterations: 1..100
- `--backend <name>` records backend selection metadata
- other `--<arg> <value>` flags are forwarded as Eng Args overrides
- `jit.status` is `not_available`
- comparison_policy is `no-speedup-claim`
```

Example:

```bat
eng.exe jit-bench examples\official\01_csv_plot\main.eng --iterations 1
```

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check/runtime setup failure
```

## `eng-ide.exe`

Launches the native portable tester IDE.

Current tester IDE features:

```text
- Explorer for examples, stdlib, tutorials, and scratch .eng files
- native source editor with EngLang syntax highlighting
- live check_source diagnostics for unsaved edits
- toolbar diagnostic counts and Problems panel
- completion insertion for keywords, quantity kinds, units, and starter snippets
- compiler-derived symbol metadata
- save/check/run commands
- generated report and plot opening
- in-IDE PlotSpec preview
- Artifacts tab for result, review, report, PlotSpec, manifest, SVG, and bytecode runtime objects, with saved paths when files are written
- Settings window for light/dark theme, UI density, font sizes, window presets,
  soft wrapping, and panel default sizes
```

`eng-ide.exe --smoke` checks the non-GUI path for release packages. It verifies
that examples are discoverable, compiler completion metadata is available, and
the official domain/component track example produces domain, component, and
connection metadata.

## `eng-lsp.exe`

Starts the experimental stdio LSP server when no flags are supplied. The release
package also supports smoke and snapshot commands:

```bat
eng-lsp.exe --smoke
eng-lsp.exe --snapshot examples\official\06_domain_port\main.eng
eng-lsp.exe --snapshot-check examples\official\01_csv_plot\main.eng
```

`--smoke` verifies the official CSV snapshot path and the official domain/component track
domain/component metadata path. `--snapshot` emits `eng-lsp-snapshot-v1` JSON
with diagnostics, completion items, and hover items. Domain/component files
include hover `kind`/`status` metadata and completion labels such as
`Thermal`, `RoomBoundary`, and `RoomBoundary.heat`.

## `eng run <file.eng> [--open-report] [--save-artifacts] [--<arg> <value>...]`

Runs the file's top-level workflow through bytecode v1 and the native VM seed.
By default, result/review/report/plot payloads remain runtime objects in memory.
`--save-artifacts` writes those objects to disk.

Execution model:

```text
1. Root `args { ... }` declares CLI-bindable arguments.
2. Top-level statements form the executable workflow.
3. `script` blocks are rejected with E-SCRIPT-001.
```

Saved artifacts:

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

`--open-report` implies `--save-artifacts` and attempts to open the generated
`report.html` with the OS default browser.

Args flags are matched against root `args { ... }` fields. Defaults are used
when available, primitive typed values are normalized, and resolved values are
recorded in `arg_values`.

Current typed Args conversion:

```text
String/Path/CsvFile/DirectoryPath  recorded as text/path
Bool/Boolean         true/false, yes/no, on/off, 1/0 -> true/false
Int/Integer          whole-number signed integer
Count/usize/u32/u64  non-negative whole-number count
Float/Number         finite numeric value
Duration             s, min, h -> normalized seconds such as `600 s`
```

```bat
eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts --input data/sensor.csv
```

## `eng build <file.eng> --standalone --profile repro`

Creates a runnable standalone package bundle:

```text
dist/
  <model>-standalone/
    eng.exe
    run.bat
    ARGS_HELP.txt
    <model>.engpkg
    <model>.lock
    <model>.engbc
    <model>.review.html
    source/
      <file.eng>
```

For CSV promotions that use relative paths, the referenced CSV files are copied
into the bundle at the same relative path from `source/<file.eng>`. Running
`run.bat` executes the bundled `eng.exe run source\<file.eng> --save-artifacts`
and forwards extra Args flags. It creates normal `build/result` artifacts inside
the bundle.

The `.engpkg` records package format, runtime ABI, repro profile, runner,
engine, source and artifact roots, source, bytecode, source hash, bytecode hash,
workflow signature, Args schema, Args field count, Args help
path, dependency count, dependency paths, and dependency hashes. The lock file
records runtime/compiler/package/bytecode/result/report/plot format versions,
source and bytecode hashes, workflow signature, dependency count, dependency hashes, and
`profile = repro`.

See [Standalone package reference](../reference/standalone_package.md) for the
full bundle layout, manifest and lock field tables, hash semantics, and the
reserved `model.exe`/AOT boundary.

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
- official user-test examples check first
- compatibility regression examples check after official examples
- unit mismatch example produces errors
- ambiguous power example produces a warning
- HeatRate sum example produces W-STATS-SUM-001
- physical equation using == produces E-EQ-BOOL-001
- equation unit mismatch produces E-EQ-UNIT-001
- missing CSV column example produces errors
- missing uncertainty source example produces E-UNC-SOURCE-001
- invalid uncertainty argument example produces E-UNC-ARGS-001/002/003
- `script` execution-root syntax produces E-SCRIPT-001
- official plotting example produces report and PlotSpec artifacts
- official histogram example produces binned PlotSpec artifacts
- Args CLI binding produces CSV run artifacts
- typed Args values are normalized and invalid typed Args values produce E-ARGS-TYPE-001
- official CSV example produces runtime optimization track kernel candidates
- bad DateTime and bad numeric CSV fixtures record parse_failures
- numeric missing interpolation fixture executes
- constraint violation fixture records upper-bound policy violation
- official simple system example produces system report artifacts
```
