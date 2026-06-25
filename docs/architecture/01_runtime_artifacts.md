# Runtime Artifacts

`eng run <file.eng>` creates reviewable runtime objects without Python.
`eng run --save-artifacts <file.eng>` writes those objects as files.

## Saved Directory Layout

```text
build/
  <source-stem>.engbc
  result/
    result.engres
    review.json
    report.html
    report_spec.json
    run_log.json
    process_results.json
    test_results.json
    output_manifest.json
    summary.csv          # only when source uses explicit CSV export
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

Current v0.9 header:

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
workflow = top_level
workflow_args = args:Args
workflow_return = Report
```

Current v0.9 sections:

```text
objects:
table|sensor|SensorData|4|<csv_hash>|9
scalar|cp|SpecificHeat|J/kg/K|10
timeseries|Q_coil|Time|HeatRate|W|11
scalar|E_coil|Energy|J|12

instructions:
0000|enter_workflow|top_level
0001|load_table|sensor
0002|load_scalar|cp
0003|load_timeseries|Q_coil
0004|load_scalar|E_coil
0005|write_result|engres-v1
```

The format is intentionally text for early review and snapshot testing. It can move to a compact binary encoding after the contract is stable.
`workflow = top_level` means the source file runs directly from top-level
statements. `script` blocks are rejected and are not represented in bytecode.

## Explicit CSV Exports

`export summary to csv "summary.csv" { ... }` writes a user-requested one-row
CSV summary record under `build/result`. This happens even when ordinary run
artifacts remain in memory, because the export statement is an explicit
artifact command.

CSV summary exports are not first-class Summary objects. The export block
assembles scalar fields such as named bindings, integration results, and scalar
statistics. Headers include display units, and cells contain formatted scalar
values. The v0.2 decision record is
[`summary_object_decision.md`](../reference/summary_object_decision.md).

Export and write outputs use idempotent overwrite hardening. Re-running with
identical generated contents succeeds. Replacing different existing contents
requires an attached `with { overwrite = true }` block.

## Explicit Write Outputs

`write text` and `write json` produce small generated output files under
`build/result`:

```eng partial
write text "outputs/run_note.txt", notes_text
write json "outputs/energy.json", E_coil
```

`write json` serializes scalar quantities as objects with `value`,
`quantity_kind`, and `unit`. Raw JSON text is passed through when the expression
already evaluates to JSON-looking text.

## Explicit File Operations

`copy`, `move`, and `delete` provide a constrained filesystem mutation seed.
Generated-output mutation targets remain under `build/result`; `move` and
`delete` require explicit confirmation metadata.

```eng partial
copy file("data/template.txt") to "ops/copied_note.txt"

move "ops/copied_note.txt" to "ops/archive/copied_note.txt"
with {
    confirm = true
    overwrite = true
}

delete "ops/scratch.txt"
with {
    confirm = true
}
```

The runtime records file operation effects as output manifest entries such as
`copy_file`, `move_file`, and `delete_file`.

## `process_results.json`

Purpose:

```text
reviewable external process execution records for IDEs, CI, and audit tooling
```

Current format:

```text
eng-process-results-v1
runtime_version
source_path
process_count
processes[].binding
processes[].command
processes[].tool_version
processes[].args
processes[].cwd
processes[].expected_outputs
processes[].expected_output_status
processes[].exit_code
processes[].success
processes[].stdout
processes[].stdout_hash
processes[].stderr
processes[].stderr_hash
processes[].duration_ms
processes[].status
processes[].line
```

`run command` statements must bind a `ProcessResult`. The compiler records the
declaration in `review.json`; the runtime writes exit status and captured
stdout/stderr plus stable stdout/stderr hashes here. `tool_version` is an
explicit owner option for workflows that need to pin or review the external
tool identity. Non-zero exits fail by default unless the owner has
`with { allow_failure = true }`.

## `test_results.json`

Purpose:

```text
runtime assertion and golden-comparison records for IDEs, CI, and review tooling
```

Current format:

```text
eng-test-results-v1
runtime_version
source_path
test_count
failed_count
tests[].name
tests[].status
tests[].line
tests[].assertions
tests[].goldens
```

`test` blocks group `assert` statements and `golden` artifact comparisons.
The compiler records the check intent in `review.json`; the runtime writes
pass/fail status, rendered messages, and source lines here. Test failures make
`eng run` fail after saved artifacts are available.

## `run_log.json`

Purpose:

```text
structured runtime message stream for IDEs, CI, and review tooling
```

Current format:

```text
eng-run-log-v1
runtime_version
source_path
message_count
messages[].index
messages[].level
messages[].message
messages[].line
```

`print` records use level `print`. `log debug`, `log info`, `log warn`, and
`log error` records use their declared level. The CLI still writes a human
stdout stream; `run_log.json` is the machine-readable companion.

## `output_manifest.json`

Purpose:

```text
listing of generated output files and content hashes
```

Current format:

```text
eng-output-manifest-v1
runtime_version
source_path
execution_profile
artifact_count
artifacts[].kind
artifacts[].path
artifacts[].hash
artifact_registry.format
artifact_registry.source_files[]
artifact_registry.generated_files[]
artifact_registry.external_commands[]
artifact_registry.network_requests[]
artifact_registry.db_writes[]
artifact_registry.model_artifacts[]
artifact_registry.caches[]
artifact_registry.tests[]
profile_diagnostics[]
```

When `--save-artifacts` is used, this manifest lists result/report/PlotSpec/SVG
files alongside explicit CSV exports, write outputs, file operation records,
run-log records, and process-result records. The saved process artifact is
listed as `process_results`; the saved test artifact is listed as
`test_results`. The companion `artifact_registry` groups the same outputs
with source records, external command boundaries, DB-write summaries, model
artifact summaries, and named test records so report, review, IDE, and CI tools
can consume one generic artifact shape.

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
  "workflow": {
    "kind": "top_level",
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
    "numeric_values": [
      {
        "binding": "Q_dist",
        "value_kind": "scalar",
        "representation": "Distribution",
        "status": "uncertainty_attached"
      }
    ],
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
        "reason": "recognized first-order thermal ODE and executed fixed-step runtime path"
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
    "environment_dependencies": [],
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
workflow
args_summary
arg_values
environment_dependencies
inferred_declarations
expected_types
hover_hints
type_info
unit_derivations
unit_conversion_table
axis_info
stats_info
integrations
prints
csv_exports
writes
file_operations
process_runs
uncertainty_info
ml_info
system_summary
system_ir
domain_summary
component_summary
connection_summary
assembly_summary
component_graph
schema_summary
schemas
csv_promotions
```

`review.json` is produced by `eng check --review` and by saved `eng run`
artifacts. The `plot_manifest` section declares the runtime plot manifest
location, while the runtime-specific manifest hash is recorded in
`report_spec.json`.

## `report_spec.json`

Purpose:

```text
machine-readable report/review contract for UI, LSP, packaging, and review tooling
```

Current v0.9 format:

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
provenance.environment_dependencies
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
assembly_summary
component_graph
plot_manifest
warning_list
```

`domain_summary.type_parameters` stores structured domain/component track metadata as objects
with `kind`, `name`, and `display`, so report/IDE/LSP consumers can distinguish
the parameter kind (`Medium`, `Frame`, `Axis`) from the local parameter name
used in a package declaration.

`assembly_summary` stores component graph connection sets, generated
across/through equation seeds, residual graph metadata, `domain_plans`,
`solver_preview`, and component-local expression counts.
`component_summary.local_expressions` keeps those component-owned
`name = expr` metadata records separate from top-level workflow bindings. A graph with more than one generated domain plan is labeled
`multi_domain_preview`; that label means reviewable metadata plus homogeneous
connection-constraint residual checking, not production physical multi-domain
solving.

`component_graph` stores the graph-shaped domain/component view used by report
and IDE consumers. It includes component nodes, port nodes, connection edges,
connection sets, domain labels, generic medium/frame/axis labels when present,
and source spans for source-linked graph navigation.

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

The v0.9 system summary records report-facing equation metadata:

```text
system name
parameter/state/input variables
equations with left/right dimensions
residual name and expression
status = unit_consistent or unit_unresolved
```

The current system/equation support also records a machine-readable `system_ir` section in
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
When `simulate` is requested for a system shape outside that runner, the same
solver result records `skipped_unsupported_shape` and no `sim.<state>`
TimeSeries is created.

## `report.html`

Purpose:

```text
browser-readable review report generated beside result.engres
```

Current sections include:

```text
summary metrics
workflow metadata
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

Current format:

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

Each manifest plot entry records the generated PlotSpec/SVG paths and the
series names included in that plot.

## `plots/*.svg`

Purpose:

```text
Python-free plot artifact generated from PlotSpec v1 by the native report crate
```

## `eng build --standalone`

The current build command creates a runnable packaged bundle:

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
workflow = top-level workflow(args: Args) -> Report
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
workflow = top-level workflow(args: Args) -> Report
dependency_count = 1
dependency_hashes = source/data/sensor.csv:<hash>
```

`run.bat --help` prints `ARGS_HELP.txt`, which is generated from root
`args { ... }` metadata when available. Extra `run.bat --<field> <value>` flags
are forwarded to `eng.exe run`, where they are bound to root args fields.

`run.bat` executes the bundled `eng.exe` and writes normal run artifacts under
`<model>-standalone/build/result`. This is a packaged runner, not an optimized
AOT executable. Full AOT/native optimization remains a later milestone.

See [Standalone package reference](../reference/standalone_package.md) for the
field tables, dependency hash semantics, Args forwarding behavior, and reserved
`model.exe` boundary.
