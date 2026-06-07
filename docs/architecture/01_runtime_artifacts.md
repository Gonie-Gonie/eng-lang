# Runtime Artifacts

`eng run <file.eng>` creates a reviewable artifact set without Python.

## Directory Layout

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

## `.engbc`

Purpose:

```text
checked source -> bytecode v1 -> native VM seed
```

Current v0.4 header:

```text
ENGBYTECODE 1
format = engbc-v1
bytecode_version = 1
compiler_version = ...
source_hash = ...
source_bytes = ...
source_lines = ...
tokens = ...
ast_items = ...
typed_bindings = ...
schemas = ...
csv_promotions = ...
entry = script main
entry_args = args:Args
entry_return = Report
```

Current v0.4 sections:

```text
objects:
table|sensor|SensorData|4|<csv_hash>|9
scalar|cp|SpecificHeat|J/kg/K|10
timeseries|Q_coil|Time|HeatRate|W|11
scalar|E_coil|Energy|J|12

instructions:
0000|enter_entry|script|main
0001|load_table|sensor
0002|load_scalar|cp
0003|load_timeseries|Q_coil
0004|load_scalar|E_coil
0005|write_result|engres-v1
```

The format is intentionally text for early review and snapshot testing. It can move to a compact binary encoding after the contract is stable.

## `.engres`

Purpose:

```text
typed VM result container for report/view/build workflows
```

Current result fields:

```json
{
  "format": "engres-v1",
  "result_format_version": 1,
  "runtime_version": "...",
  "compiler_version": "...",
  "bytecode_version": 1,
  "source_path": "...",
  "source_hash": "...",
  "bytecode_hash": "...",
  "numeric_profile": "preview-f64",
  "entry": {
    "kind": "script",
    "name": "main",
    "arg_name": "args",
    "arg_type": "Args",
    "return_type": "Report"
  },
  "arg_values": [
    {
      "name": "input",
      "type": "String",
      "value": "data/sensor.csv",
      "source": "default"
    }
  ],
  "object_store": {
    "scalar_count": 2,
    "table_count": 1,
    "timeseries_count": 1,
    "array_count": 0,
    "objects": [
      {
        "kind": "table",
        "columns": [
          {
            "unit": "degC",
            "canonical_unit": "K",
            "values": [7.1],
            "canonical_values": [280.25],
            "conversion_failures": []
          }
        ],
        "parse_failures": []
      },
      {
        "kind": "timeseries",
        "points": []
      }
    ]
  },
  "typed_payload": {
    "kind": "Report",
    "status": "ok",
    "result_format": "engres-v1",
    "vm_steps": [],
    "statistics": [
      {
        "status": "computed",
        "statistics": []
      }
    ],
    "integrations": [
      {
        "status": "computed",
        "method": "trapezoidal"
      }
    ],
    "uncertainties": [
      {
        "kind": "Distribution",
        "status": "sampled_seed",
        "sample_count": 31
      }
    ],
    "ml": [
      {
        "kind": "RegressionModel",
        "status": "trained_seed",
        "rmse": 41.79,
        "r2": 0.94
      }
    ],
    "policy_results": [
      {
        "kind": "constraint",
        "status": "executed",
        "violation_count": 0
      }
    ],
    "systems": [],
    "solver_boundaries": [
      {
        "system": "RoomThermal",
        "status": "computed",
        "reason": "recognized first-order thermal ODE and executed fixed-step preview"
      }
    ],
    "system_ir": [
      {
        "system": "RoomThermal",
        "equations": [
          {
            "residual": "RoomThermal.residual_1",
            "dependencies": [],
            "derivative_states": ["T"]
          }
        ]
      }
    ]
  },
  "provenance": {
    "schema_count": 1,
    "csv_promotion_count": 1,
    "system_count": 1,
    "equation_count": 1,
    "residual_count": 1,
    "data_hashes": [],
    "unit_conversion_history": [],
    "plot_spec_hash": "...",
    "report_spec_hash": "...",
    "schema_hash": "preview"
  }
}
```

## `review.json`

Purpose:

```text
semantic review artifact for humans, tooling, and LLM-assisted code review
```

Current sections:

```text
review_schema_version
syntax_summary
quantity_completion_count
diagnostics
variable_table
warning_list
plot_manifest
entry_points
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
uncertainty_info
ml_info
system_summary
system_ir
domain_summary
component_summary
connection_summary
schema_summary
schemas
csv_promotions
```

`review.json` is produced by both `eng check --review` and `eng run`. The `plot_manifest` section declares the runtime plot manifest location, while the runtime-specific manifest hash is recorded in `report_spec.json`.

## `report_spec.json`

Purpose:

```text
machine-readable report/review contract for UI, LSP, packaging, and review tooling
```

Current v0.8 format:

```text
eng-report-spec-v1
report_schema_version = 1
```

Current sections:

```text
provenance
variable_table
inferred_declaration_table
unit_conversion_table
args_summary
arg_values
schema_summary
computed_statistics
computed_integrations
uncertainty
ml
policy_results
system_summary
system_ir
domain_summary
component_summary
connection_summary
plot_manifest
warning_list
```

`domain_summary.type_parameters` stores structured domain/component track metadata as objects
with `kind`, `name`, and `display`, so report/IDE/LSP consumers can distinguish
the parameter kind (`Medium`, `Frame`, `Axis`) from the local parameter name
used in a package declaration.

The uncertainty track section records declared uncertainty forms, deterministic
runtime summaries when available, scale/offset transforms, and propagation
source terms with source, role, and quantity kind.

The plot manifest section records:

```text
path = plots/plot_manifest.json
hash = <plot_manifest_hash>
format = eng-plot-manifest-v1
plot_count = 1
```

The v0.8 system summary records report-facing equation metadata:

```text
system name
parameter/state/input variables
equations with left/right dimensions
residual name and expression
status = unit_consistent or unit_unresolved
```

The current system/equation preview also records a machine-readable `system_ir` section in
`review.json` and `report_spec.json`:

```text
system name
solver_boundary.status = unsolved
solver_boundary.reason
parameter/state/input/equation/residual counts
solver_plan.status = metadata_only
source-order solve_order residual list
ODE runner status = deferred
Jacobian seed state columns per residual
equation relation and normalized residual
parameter/state/input dependencies per residual
derivative state mentions
```

`review.json` remains compiler-only and records the unsolved metadata boundary.
During `eng run`, `report_spec.json` and `result.engres` can upgrade the same
system to `computed` for the official one-state thermal ODE. The result payload
then records `solver_result` with method `explicit_euler_fixed_step`, state
trajectory points, step count, time step, and final state value.

## `report.html`

Purpose:

```text
browser-readable review report generated beside result.engres
```

Current sections include:

```text
summary metrics
entry points
Args metadata
inferred declarations
hover hints
type info
unit derivations
axis info
statistics
integrations
system equations
schemas
CSV promotions
diagnostics
SVG plot iframe
```

## `plots/plot_spec.json`

Purpose:

```text
interactive-friendly plot data model consumed by native renderers/viewers
```

Current preview format:

```text
eng-plotspec-v1
line plot
x/y axis labels with units
CSV-derived TimeSeries points for the official data path
```

The uncertainty track path keeps the same PlotSpec version and adds optional
series `bins` for `plot distribution(...)` histograms. Each bin records lower
edge, upper edge, center, and count while `points` remains center/count data for
older renderers.

## `plots/plot_manifest.json`

Purpose:

```text
listing of generated plot outputs and hashes
```

`eng view <result.engres>` prints this path when it exists.

## `plots/*.svg`

Purpose:

```text
Python-free plot artifact generated from PlotSpec v1 by the native report crate
```

## `eng build --standalone`

The current preview build command creates a runnable packaged bundle:

```text
dist/
  <model>-standalone/
    eng.exe
    run.bat
    <model>.engbc
    <model>.engpkg
    <model>.lock
    ARGS_HELP.txt
    <model>.review.html
    source/
      <file.eng>
```

The `.engpkg` records:

```text
format = engpkg-stable-1
package_format_version = 1
runtime_abi = eng-runtime-cli-v1
profile = repro
runner = run.bat
engine = eng.exe
source_root = source
artifact_root = build/result
source = source/<file.eng>
bytecode = <model>.engbc
source_hash = ...
bytecode_hash = ...
entry_name = main
entry = script main(args: Args) -> Report
args_schema = Args
args_field_count = 1
args_help = ARGS_HELP.txt
dependency_count = 1
dependencies = source/data/sensor.csv
dependency_hashes = source/data/sensor.csv:<hash>
```

The `.lock` records:

```text
runtime_version = ...
compiler_version = ...
package_format_version = 1
runtime_abi = eng-runtime-cli-v1
bytecode_version = 1
result_format_version = 1
report_schema_version = 1
plot_spec_version = 1
profile = repro
source_hash = ...
bytecode_hash = ...
entry_name = main
dependency_count = 1
dependency_hashes = source/data/sensor.csv:<hash>
```

`run.bat --help` prints `ARGS_HELP.txt`, which is generated from `struct Args`
metadata when available. Extra `run.bat --<field> <value>` flags are forwarded
to `eng.exe run`, where they are bound to `struct Args` fields.

`run.bat` executes the bundled `eng.exe` and writes normal run artifacts under
`<model>-standalone/build/result`. This is a packaged runner, not an optimized
AOT executable. Full AOT/native optimization remains a later milestone.

See [Standalone package reference](../reference/standalone_package.md) for the
field tables, dependency hash semantics, Args forwarding behavior, and reserved
`model.exe` boundary.
