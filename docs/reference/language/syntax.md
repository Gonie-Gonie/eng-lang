# EngLang Language Grammar Guide

This guide is the practical grammar reference for the current EngLang package scope.
It is written for someone who wants to open an `.eng` file, understand the
shape of the language, and write a small engineering workflow without reading
the compiler source.

The current public package documents the data-to-report workflow. Sections
outside that package scope mark themselves as Supported, Internal, or Planned.

## What To Read First

If you are new to EngLang, read these sections in order:

1. Execution model
2. Anatomy of a useful file
3. Declarations and expressions
4. Built-in command-style verbs
5. `where` and `with`
6. Print, log, export, report, and artifacts

For a concrete runnable example, open:

```text
examples/official/09_command_where_with/main.eng
```

That example combines schema input, top-level execution, command-style
statistics/integration, scoped `where` locals, `with` options, unit-aware
printing, CSV export, and report plotting.

For the integrated data plus simulation workflow, open:

```text
examples/internal/17_measured_vs_simulated/main.eng
```

That example promotes measured and weather CSV files, runs a minimal thermal
simulation, computes RMSE, validates a threshold, and plots measured plus
simulated TimeSeries together.

## Execution Model

EngLang executes one source file as a top-level workflow.

There is no public `entry` selector. There is no `script main` execution root.
The body of the file is the workflow. Command-line inputs are declared with one
root `args { ... }` block.

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
}

sensor = promote csv args.input as SensorData
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
E_coil = integrate Q_coil over Time

print "E total = {E_coil: .2 kWh}"
```

The compiler reads the source in order and builds:

| Layer | What It Means |
|---|---|
| Parsed program | Lines, tokens, AST items, source spans, block context |
| Semantic model | Typed bindings, schemas, functions, diagnostics, review metadata |
| Bytecode | Native runtime plan for the top-level workflow |
| Runtime data | Tables, TimeSeries, integrations, statistics, outputs |
| Report artifacts | `result.engres`, `review.json`, `report_spec.json`, `run_log.json`, `process_results.json`, `test_results.json`, PlotSpec, SVG, HTML |

Use these commands from the repository or portable package:

```text
eng.exe check examples/official/09_command_where_with/main.eng --review
eng.exe run examples/official/09_command_where_with/main.eng --save-artifacts
eng.exe run examples/internal/17_measured_vs_simulated/main.eng --profile repro --save-artifacts
eng.exe view build/result/result.engres
```

## Anatomy Of A Useful File

A typical data-to-report file has this order:

```eng partial
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}

args {
    input: CsvFile = file("data/sensor.csv")
}

sensor = promote csv args.input as SensorData
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
E_coil = integrate Q_coil over Time
mean_Q = mean Q_coil over Time

print "Loaded {sensor.rows} rows from {args.input}"
print "Q mean = {mean_Q: .2 kW}"
log info "E total = {E_coil: .2 kWh}"
log warn "review load profile before publishing report"

export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
    E_coil as kWh with ".2"
}

report {
    summarize Q_coil by [mean, max, p95]
    plot Q_coil over Time
    with {
        unit y = kW
        title = "Coil heat rate"
    }
}
```

The file reads as: declare the input shape, bind CLI arguments, promote a CSV
into typed data, compute quantities, print quick CLI output, record structured
runtime messages, export a durable summary CSV, and ask the report system for
reviewable artifacts.

## Lexical Basics

EngLang source files are UTF-8 text files.

Comments use `//`:

```eng partial
Q = 10 kW // design heat rate
```

Strings use double quotes:

```eng partial
label = "case A"
print "case = {label}"
```

Identifiers are ASCII-style names:

```text
Q_coil
mean_Q
sensor
T_return
args.input
```

Units may contain `/` and may be written after numeric literals:

```eng partial
L = 2 m
Q = 10 kW
cp = 4180 J/kg/K
m_dot = 0.22 kg/s
T = 21.4 degC
```

`degC` is the canonical ASCII spelling. `°C` is accepted as a user-facing alias
for absolute temperature and display formatting.

## Top-Level Forms

The current top-level declaration families are:

| Form | Example | Notes |
|---|---|---|
| File import | `use "thermal.eng"` | Imports functions and importable constants |
| Package import metadata | `import package.name` | Declared metadata path, not full package manager |
| Args block | `args { input: CsvFile = file("data.csv") }` | Root CLI argument schema |
| Const declaration | `const cp: SpecificHeatCapacity = 4180 J/kg/K` | Importable when pure |
| Explicit declaration | `E: Energy [J] = 3600 J` | Public type and display unit boundary |
| Fast binding | `Q = 10 kW` | Inferred declaration |
| Schema | `schema SensorData { ... }` | CSV/data boundary |
| Function | `fn heat_loss(...) -> HeatRate [W] { ... }` | Typed scalar helper support |
| System | `system Room { ... }` | Supported one-state thermal and two-state source-equation run support; internal state-space vector path |
| Domain/component | `domain Fluid { ... }` | Internal metadata track |
| Print | `print "Q = {Q: .2 kW}"` | Debug/CLI output |
| Log | `log warn "Q is high"` | Structured runtime message |
| CSV export | `export summary to csv "summary.csv" { ... }` | Durable scalar artifact |
| Write output | `write text "note.txt", note` | Explicit generated output |
| File operation | `copy file("note.txt") to "outputs/note.txt"` | Explicit output-area file mutation |
| Process run | `result = run command "cmd"` | Explicit external process capture as `ProcessResult` |
| Test block | `test "summary" { assert Q > 0 kW }` | Runtime assertion and golden checks |
| Report | `report { plot Q over Time }` | Review/report artifact requests |

Rejected compatibility forms:

```eng error
script main {
    Q = 10 kW
}
```

```eng error
struct Args {
    input: CsvFile
}
```

Use root `args { ... }` and top-level workflow statements instead.

## Args

`args { ... }` declares user-provided inputs. The block belongs at top level.

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
    case_name: String = "baseline"
    api_key: Secret[String] = secret env("API_KEY")
    enabled: Bool = true
    count: Count = 3
    gain: Float = 1.0
    window: Duration = 10 min
}
```

Supported argument types include:

| Type | Example Default | CLI Shape |
|---|---|---|
| `String` | `"baseline"` | `--case_name demo` |
| `Path` / `FilePath` / `CsvFile` | `file("data/sensor.csv")` | `--input data/other.csv` |
| `DirectoryPath` | `dir("runs")` | `--output runs/case1` |
| `Secret[T]` | `secret env("API_KEY")` | `--api_key <value>` |
| `Bool` | `true` | `--enabled true` |
| `Int` / `Integer` / `Count` | `3` | `--count 12` |
| `Float` / `Number` | `1.0` | `--gain 1.25` |
| `Duration` | `10 min` | `--window 30 s` |

Runtime records the final bound value and whether it came from the default or
CLI override. `Secret[T]` values are recorded as `<redacted>` with
`redacted: true`; the inner `T` controls argument normalization.

## Path Helpers And Exists

Path defaults are typed. Use `file("...")` for file-like arguments and
`dir("...")` for directory-like arguments.

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
    output: DirectoryPath = dir("build/result")
}
```

Pure path helpers are allowed in top-level bindings and print formatting:

```eng partial
summary_file = join(args.output, "summary.csv")
input_parent = parent(args.input)
input_name = stem(args.input)
input_ext = extension(args.input)

print "summary target = {summary_file}"
print "input source = {input_parent}/{input_name}.{input_ext}"
```

`exists` is intentionally not pure. It queries the filesystem at check/run time,
returns `Bool`, and is recorded as environment dependency provenance:

```eng partial
input_exists = exists args.input
print "input exists = {input_exists}"
```

The current runtime resolves relative paths against the source file directory.
For `examples/official/10_path_policy/main.eng`, the review/result/report-spec
artifacts contain an `environment_dependencies` entry for `exists args.input`.

## Read-Only I/O

Use read-only I/O when a workflow needs small UTF-8 text/config companion files
in addition to typed engineering data. The current runtime returns raw strings;
JSON and TOML are not yet parsed into structured EngLang objects.

```eng partial
args {
    notes: TextFile = file("data/notes.txt")
    config_json: JsonFile = file("data/case.json")
    config_toml: TomlFile = file("data/case.toml")
}

notes_text = read text args.notes
json_text = read json args.config_json
toml_text = read toml args.config_toml

print "notes = {notes_text}"
print "json config = {json_text}"
print "toml config = {toml_text}"
```

Parenthesized call forms are equivalent:

```eng partial
notes_text = read_text(args.notes)
json_text = read_json(args.config_json)
toml_text = read_toml(args.config_toml)
```

Read-only I/O rules:

| Rule | Meaning |
|---|---|
| Paths resolve source-relative | `file("data/notes.txt")` is resolved beside the `.eng` file |
| Files are read as UTF-8 | Binary reads are deferred |
| Values are strings | `read json/toml` records source text, not structured values |
| JSON field access is rejected | `payload.field` on a `read json` binding emits `E-IO-JSON-FIELD-ACCESS-001`; promote to a schema first |
| Source hashes are recorded | `review.json`, `result.engres`, and `report_spec.json` include provenance |
| Hidden imported reads are rejected | Importable const/function files must not hide runtime I/O |

The runnable example is:

```text
examples/official/11_read_only_io/main.eng
```

## Typed Config Promotion

JSON/TOML config files can be promoted against a schema. Top-level, nested
object, and array/list fields are validated for required/unknown/type/null
policy. `Optional[T]` or `T?` allows a config field to be missing or set to JSON
null, and `field: Type = default` supplies a schema default for missing config
fields:

```eng partial
schema WorkflowConfig {
    year: Int
    region: Optional[String]
    output: DirectoryPath? = dir("build/out")
}

config = promote json file("data/workflow.json") as WorkflowConfig
```

The result/review artifacts record `optional_fields`,
`optional_missing_fields`, `optional_null_fields`, `nested_object_fields`, and
`array_fields`, `default_fields`, and `defaulted_fields` in the config promotion
entry.

## Schemas And CSV Promotion

Schemas are the data boundary. They describe columns, quantity kinds, display
units, and index roles.

```eng partial
schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]

    constraints {
        time is monotonic
        T_supply between 0 degC and 60 degC
        T_return between 0 degC and 80 degC
        m_dot >= 0 kg/s
    }

    missing {
        T_supply: interpolate max_gap=10 min
        T_return: interpolate max_gap=10 min
        m_dot: error
    }
}
```

Promote a CSV into a typed table:

```eng partial
sensor = promote csv args.input as SensorData
```

After promotion:

| Expression | Meaning |
|---|---|
| `sensor.rows` | Row count for print/export formatting |
| `sensor.T_supply` | Typed column expression |
| `sensor.T_return` | Typed column expression |
| `sensor.m_dot` | Typed numeric column expression |

The official HeatRate path recognizes expressions like:

```eng partial
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

This produces `TimeSeries[Time] of HeatRate` metadata and runtime values when
the source table has a DateTime index and the required numeric columns.

Schema-aware table transforms are supported for filtering promoted tables
and requiring exactly one row:

```eng partial
candidates = filter stations
where {
    region == args.region
    valid_from <= date(args.year, 1, 1)
    valid_to is none or valid_to >= date(args.year, 12, 31)
}

station_fields = select stations columns station_id, latitude
station_plus = derive stations column longitude_copy = longitude
stations_by_latitude = sort stations by latitude desc

station = require_one candidates
with {
    on_none = error "No station for region/year"
    on_many = error "Multiple stations for region/year"
}

joined = join samples with results
on {
    samples.case_id == results.case_id
}
```

The compiler validates referenced predicate, selected, derived-source, and
sort-key columns against the promoted table schema, rejects obvious predicate
literal type mismatches with `E-TABLE-PREDICATE-TYPE`, and validates join keys
against both promoted table schemas. Date/DateTime predicates compare ISO
timestamp strings by UTC instant when both sides include time zones, and compare
date-only values by their `YYYY-MM-DD` prefix, including values produced by
`date(year, month, day)`. It records the transform contract in
`review_document.table_transforms[]`.
Runtime artifacts record row counts, selected columns, derived columns, sort
keys, predicate evidence, join key pair counts, and row diagnostics in
`typed_payload.table_transforms[]`.

Generate deterministic sample tables:

```eng partial
samples = sample lhs
with {
    count = 100
    seed = 42
    cooling_cop = uniform(2.5, 5.0)
    lighting_power_density = uniform(5 W/m2, 15 W/m2)
}
```

Supported methods are `grid`, `random`, and `lhs`. Generated tables include a
`case_id` column, parameter columns, row hashes, generation metadata, and seed
metadata in `typed_payload.sample_tables[]`. `random` and `lhs` use deterministic
seeded generation; repro profile rejects them without `seed`.

Sample-like tables with a `case_id` column also materialize case artifacts.
Runtime output includes `typed_payload.case_tables[]` summary rows,
`typed_payload.case_manifests[]` per-case rows, and
`typed_payload.case_diagnostics[]` for duplicate IDs, case-directory collisions,
missing outputs, failed steps, and cache skips. Case statuses use
`pending`, `succeeded`, `failed`, and `skipped`.

## Declarations

### Fast Bindings

Fast bindings use `=` and rely on inference:

```eng partial
Q = 10 kW
T = 21.4 degC
E = 3.6 kWh
eta = 0.82
```

The compiler can infer common physical quantities from the variable name and
unit. Ambiguous units produce warnings. For example `power = 10 kW` may be
HeatRate, ElectricPower, or MechanicalPower; use an explicit declaration when
you need to lock it down.

### Explicit Declarations

Explicit declarations define the quantity kind and optional display unit:

```eng partial
E_total: Energy [kWh] = 3.6 kWh
Q_design: HeatRate [kW] = 10 kW
T_room: AbsoluteTemperature [degC] = 22 degC
```

Use explicit declarations at public boundaries, where unit/quantity ambiguity
would otherwise make review harder.

### Constants

Constants are intended for reusable pure values:

```eng partial
const cp_water: SpecificHeatCapacity = 4180 J/kg/K
const eta_nominal: Ratio = 0.864
```

Importable constants can be shared by `use "file.eng"`. Constants should not
depend on runtime inputs, side effects, or `args`.

## Expressions

The current expression support is intentionally small and quantity-aware. The
common supported shapes are:

| Shape | Example |
|---|---|
| Literal with unit | `10 kW` |
| Binding reference | `Q_coil` |
| Args field | `args.input` |
| Table row count | `sensor.rows` |
| Read-only I/O | `read text args.notes` |
| Arithmetic | `m_dot * cp * (T_return - T_supply)` |
| Function call | `heat_loss(UA, dT)` |
| Built-in call | `mean(Q_coil, axis=Time)` |
| Command-style built-in | `mean Q_coil over Time` |

Arithmetic is checked for obvious quantity errors. Adding physical quantities
to dimensionless literals is rejected:

```eng error
L = 1 m + 2
```

Write units explicitly:

```eng partial
L = 1 m + 2 m
```

## Functions And Imports

Use functions for reusable scalar calculations. Function calls remain
parenthesized.

```eng partial
fn heat_loss(UA: ThermalConductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {
    Q = UA * dT
    return Q
}
```

Function rules:

| Rule | Meaning |
|---|---|
| Typed parameters | Each parameter has a quantity/scalar type and optional unit |
| Explicit return type | The function declares its output quantity |
| One return expression | Functions currently use one `return ...` |
| Function locals | Local bindings are scoped to the function body |
| Unit-checked return | Return expression dimension must match the annotation |

Import a file:

```eng partial
use "thermal.eng"

UA_wall = 150 W/K
dT_wall = 8 K
Q_wall = heat_loss(UA_wall, dT_wall)
```

Imported files may provide functions and importable constants. Their top-level
workflow body is not imported into the caller. This avoids hidden executable
side effects.

## Built-In Function Calls

Parenthesized built-in calls are always acceptable:

```eng partial
mean_Q = mean(Q_coil, axis=Time)
peak_Q = max(Q_coil, axis=Time)
E_coil = integrate(Q_coil, over=Time)
```

The command-style forms below lower to these canonical call strings. If you
are unsure which form to use, use the parenthesized call; it is the least
ambiguous.

## Parenthesis-Light Commands

Parenthesis-light syntax is reserved for built-in workflow verbs. It is not a
general replacement for function-call parentheses.

Supported command-style verbs in the current release:

| Verb | Typical Use | Canonical Shape |
|---|---|---|
| `integrate` | HeatRate over Time to Energy | `integrate(Q, over=Time)` |
| `mean` | TimeSeries mean | `mean(Q, axis=Time)` |
| `max` | TimeSeries maximum | `max(Q, axis=Time)` |
| `min` | TimeSeries minimum | `min(Q, axis=Time)` |
| `duration` | Duration-style report metadata | `duration(T, above=...)` |
| `plot` | Report plot request | `plot(Q, over=Time)` metadata |
| `show` | Report/display request metadata | `show(value)` metadata |
| `validate` | Validation request metadata | `validate(target)` metadata |
| `rmse` | Measured-vs-simulated metric | `rmse(left, right)` metadata |

Command clauses recognized by the parser:

```text
over
by
as
above
below
between
from
to
with
```

Examples:

```eng partial
E_coil = integrate Q_coil over Time
mean_Q = mean Q_coil over Time
peak_Q = max Q_coil over Time
low_Q = min Q_coil over Time
```

Lowering:

```text
integrate Q_coil over Time -> integrate(Q_coil, over=Time)
mean Q_coil over Time      -> mean(Q_coil, axis=Time)
max Q_coil over Time       -> max(Q_coil, axis=Time)
min Q_coil over Time       -> min(Q_coil, axis=Time)
```

### Validate Commands

`validate` accepts a comparison expression that evaluates to Bool:

```eng partial
rmse_T = rmse measured.T_zone vs sim.T_zone
validate rmse_T < 5 K
```

The compiler resolves both sides of the comparison and checks physical
dimensions before runtime. A missing comparison operator is rejected with
`E-VALIDATE-BOOL-001`, unresolved values are rejected with
`E-VALIDATE-EXPR-001`, and incompatible units are rejected with
`E-VALIDATE-UNIT-001`.

### Command Target Rule

Simple targets may omit parentheses:

```eng partial
E = integrate Q_coil over Time
```

Complex targets must be parenthesized:

```eng partial
E = integrate (Q_sensible + Q_latent) over Time
```

This is rejected:

```eng error
Q_sensible = 1 kW
Q_latent = 2 kW
E = integrate Q_sensible + Q_latent over Time
```

The diagnostic is `E-CMD-AMBIG-001`. The compiler asks for parentheses because
otherwise the target and clauses become hard to read and easy to misparse.

### Function Calls Do Not Become Commands

Do not write user functions in command style:

```eng partial
// Wrong design direction; general function calls stay parenthesized.
Q_wall = heat_loss UA_wall dT_wall
```

Use:

```eng partial
Q_wall = heat_loss(UA_wall, dT_wall)
```

This keeps command syntax a small workflow convenience instead of turning the
whole expression language into a whitespace parser.

## `where` Blocks

`where` introduces local calculations for the immediately preceding owner
expression or command.

```eng partial
E_coil = integrate Q_for_energy over Time
where {
    Q_for_energy = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
}
```

In this example:

| Name | Scope |
|---|---|
| `E_coil` | Top-level binding |
| `Q_for_energy` | Visible only to `E_coil` and later locals inside the same `where` block |

`where` is useful when a calculation is important to understand the owner but
should not become part of the top-level variable table.

### Where Binding Order

Where locals are source ordered:

```eng partial
E = integrate Q2 over Time
where {
    Q1 = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
    Q2 = Q1
}
```

Forward references are rejected:

```eng error
E = integrate Q2 over Time
where {
    Q2 = Q1
    Q1 = 1 kW
}
```

The diagnostic is `E-WHERE-FWD-001`.

### Where Locals Do Not Escape

This is rejected:

```eng error
E = integrate Q_local over Time
where {
    Q_local = 1 kW
}
print "Q = {Q_local: .2 kW}"
```

The diagnostic is `E-NAME-LOCAL-001`. If a value should be printed, exported,
or reused in multiple expressions, make it a top-level binding instead.

## `with` Blocks

`with` introduces options for the immediately preceding owner expression or
command. It is for configuration/display/backend choices, not for local
calculation.

```eng partial
E_coil = integrate Q_for_energy over Time
where {
    Q_for_energy = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
}
with {
    method = trapezoidal
}
```

Common accepted option keys:

| Key | Typical Meaning |
|---|---|
| `method` | Numerical/statistical method choice |
| `backend` | Execution/backend metadata choice |
| `title` | Display/report title |
| `type` | Plot type or command subtype |
| `unit x` | X-axis display unit |
| `unit y` | Y-axis display unit |
| `display_unit` | Scalar display unit |
| `solver` | Solver metadata choice |
| `tolerance` | Solver/numeric tolerance |
| `max_iter` | Solver/numeric iteration limit |
| `seed` | Deterministic seed metadata |
| `uncertainty` | Uncertainty propagation policy metadata |
| `samples` | Positive sample count for uncertainty propagation policy |
| `sensor_std` | Pointwise TimeSeries sensor standard deviation metadata |
| `output` | Artifact/output choice |
| `overwrite` | Allow replacing changed generated output content |
| `confirm` | Required confirmation for destructive file operations |
| `recursive` | Required for directory delete operations |
| `args` | External process argument array |
| `cwd` | External process working directory |
| `env` | Portable environment variables passed to the process |
| `tool_version` | Explicit external tool version metadata |
| `expected_outputs` | Process output files that must exist after the command exits |
| `timeout` | Positive process timeout duration |
| `retry` | Retry count from 0 to 5 for failed, timed-out, or missing-output attempts |
| `allow_failure` | Record a non-zero process exit instead of failing the run |

Unknown options are rejected with `E-WITH-OPTION-001`. When a block declares
`uncertainty`, the accepted policy names are `linear`, `interval`,
`monte_carlo`, and `ensemble`; `samples` must be a positive integer; and `seed`
must be a non-negative integer. `monte_carlo` without `seed` is accepted with a
reproducibility warning.

Display units are checked when the owner type is known:

```eng error
Q = 1 kW
with {
    unit y = m
}
```

The diagnostic is `E-WITH-UNIT-001`, because HeatRate cannot be displayed as
Length.

### Plot Options With `with`

Inside a `report` block:

```eng partial
report {
    plot Q_coil over Time
    with {
        unit y = kW
        title = "Coil heat rate"
    }
}
```

Multi-series line plots are supported when the series share a Time axis:

```eng partial
report {
    plot measured_data.T_zone and sim.T_zone over Time
    with {
        unit y = degC
        title = "Measured vs simulated zone temperature"
    }
}
```

This keeps the plot request readable while keeping display details grouped
below it.

The older block-style plot option form is accepted for compatibility:

```eng partial
report {
    plot Q_coil over Time {
        unit y = kW
        title = "Coil heat rate"
    }
}
```

## Print

`print` is for direct debugging and CLI output. It is not the durable artifact
path.

```eng partial
print "Loaded {sensor.rows} rows from {args.input}"
print "Q mean = {mean_Q: .2 kW}"
print "E total = {E_coil: .2 kWh}"
```

Format fields look like:

```text
{expression}
{expression: .2 unit}
{expression: .1 degC}
```

Formatting policy:

| Rule | Meaning |
|---|---|
| Expressions are type-checked | Unknown names produce diagnostics |
| Requested units are checked | `Q: .2 kW` is valid for HeatRate |
| Quantities print with units | Runtime output includes display units |
| Tables print summaries | Example: row and column summary |
| TimeSeries print summaries | Use plot/report/export for durable detail |

Examples:

```eng partial
print "Q = {Q: .2 kW}"
print "T = {T_room: .1 degC}"
print "E = {E_total: .2 kWh}"
print "eta = {eta: .3}"
```

## Log

`log <level>` is for structured runtime messages. It uses the same interpolation
and unit-checking policy as `print`, but saved runs also write the messages to
`build/result/run_log.json` for IDEs, CI tools, and reviewers.

```eng partial
log info "Q mean = {mean_Q: .2 kW}"
log warn "review high load case"
log debug "daily energy raw = {E_day: .2 kWh}"
log error "operator acknowledgement required"
```

Supported levels:

```text
debug
info
warn
error
```

`warn "..."` is intentionally not a separate command. Use
`log warn "..."` so all structured runtime messages share one form.

Runtime stdout prefixes structured messages:

```text
[info] Q mean = 5.07 kW
[warn] review high load case
```

Saved artifacts include:

```text
build/result/run_log.json
```

Each message record includes level, rendered message, source line, and
source-order index.

## Export Summary To CSV

`export summary to csv` writes a one-row scalar summary under `build/result`.

```eng partial
export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
    peak_Q as kW with ".2"
    E_coil as kWh with ".2"
}
```

Field grammar:

```text
expression as display_unit with "format"
```

The current runtime supports scalar values, statistics, integration results,
function-call scalar outputs, and typed constants. It does not yet implement a
first-class Summary object model or broad table/TimeSeries CSV export.

Attach overwrite policy when a run is allowed to replace an existing file with
different contents:

```eng partial
export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
}
with {
    overwrite = true
}
```

If the existing file has identical contents, EngLang treats the run as
idempotent and accepts it without requiring overwrite. If the contents differ,
the run fails unless `overwrite = true` is attached to the export owner.

If you need a reusable scalar value in export, bind it first:

```eng partial
mean_Q = mean Q_coil over Time
export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
}
```

## Write Outputs

Use `write` for small explicit generated files that are not better represented
as a report, plot, or scalar CSV summary.

```eng partial
write text "outputs/run_note.txt", notes_text
with {
    overwrite = true
}

write json "outputs/energy.json", E_coil
with {
    overwrite = true
}
```

Current write forms:

| Form | Output |
|---|---|
| `write text "path.txt", expression` | Text formatting of a checked expression |
| `write json "path.json", expression` | JSON string, raw JSON text, or scalar quantity object |
| `write standard_text table with { output = "path.txt" }` | Deterministic schema-aware table text artifact |

Rules:

| Rule | Meaning |
|---|---|
| Top-level only | Hidden imported writes are not supported |
| Target is under `build/result` | Absolute paths and `..` output traversal are rejected |
| Expression is checked | Unknown write expressions produce diagnostics/errors |
| Changed overwrite is explicit | Different existing content requires `with { overwrite = true }` |
| Identical reruns are accepted | Same contents can be regenerated without churn |

Generated output files are listed in:

```text
build/result/output_manifest.json
```

The runnable example is:

```text
examples/official/12_write_output_manifest/main.eng
```

## SQLite DB Writes

Use `open sqlite` plus `write <table> to db.table("...")` when a typed table
must be materialized as a reviewable SQLite side effect under `build/result`.

```eng partial
schema SimulationResult {
    case_id: String
    annual_electricity: Energy [kWh]
}

results = promote csv file("data/results.csv") as SimulationResult
db = open sqlite file("outputs/results.sqlite")

write results to db.table("simulation_results")
with {
    mode = upsert
    key = case_id
    transaction = commit
}
```

Current DB write options:

| Option | Meaning |
|---|---|
| `mode` | `append`/`insert`, `upsert`, or `replace`; default is `append` |
| `key` | Upsert key column name or list of column names |
| `transaction` | `commit` or `rollback`; default is `commit` |

Rules:

| Rule | Meaning |
|---|---|
| Top-level only | Imported files cannot hide DB writes |
| SQLite output boundary | Database files live under `build/result` |
| Typed source table | Source must be a promoted, generated, or derived typed table |
| Schema preflight | Existing SQLite table columns must match the source schema unless `mode = replace` recreates the target table |
| Reviewable records | `result.engres` and `review.json` include `db_manifests[]` |
| Manifest records | `output_manifest.json` records `sqlite_database` and `db_write_manifest` entries |

The DB write manifest records database path, hash before/after, transaction
status, schema status, diagnostics, table name, mode, key columns, schema
columns, and row count. The runtime also writes `_eng_schema_metadata` with
type/unit metadata for each exported source column.

## Template Rendering

Use `render template` when an external simulator or adapter needs a generated
text input file. Templates are source-relative UTF-8 files. Rendered outputs
and their sidecar manifests are written under `build/result`.

```eng partial
input_file = render template file("model/base_template.txt")
with {
    values = { case_id = "case_001", load = 12000 W }
    output = "outputs/case_001/input.txt"
    missing = error
}
```

Supported placeholder forms:

| Form | Meaning |
|---|---|
| `{{name}}` | Insert the named value from `with { values = { ... } }` |
| `{{name: unit}}` | Insert a numeric value formatted in the requested unit |

Rules:

| Rule | Meaning |
|---|---|
| Output boundary | Rendered files and `.render_manifest.json` sidecars stay under `build/result` |
| Missing policy | `missing = error`, `keep`, or `empty`; default is `error` |
| Hashes | The manifest records template, values, and rendered output hashes |
| Reviewable records | `result.engres` and `review.json` include `render_manifests[]` |

## File Operations

Use file operations when a workflow needs a small, explicit filesystem mutation
that is still reviewable. The current runtime keeps the mutation boundary
constrained: targets live under `build/result`, and destructive operations must
be confirmed.

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

Current file operation forms:

| Form | Meaning |
|---|---|
| `copy source to destination` | Copy a source-relative UTF-8 text file or generated output into `build/result` |
| `move source to destination` | Move a generated output path under `build/result` |
| `delete target` | Delete a generated output path under `build/result` |

Rules:

| Rule | Meaning |
|---|---|
| Top-level only | Imported files cannot hide filesystem mutations |
| Output boundary | Move/delete operate only under `build/result` |
| Confirmation | `move` and `delete` require `with { confirm = true }` |
| Directory deletion | `delete dir(...)` also requires `recursive = true` |
| Overwrite | Changed destination contents require `overwrite = true` |
| Reviewable records | `review.json` includes `file_operations[]` |
| Manifest records | `output_manifest.json` records `copy_file`, `move_file`, and `delete_file` entries |

The runnable example is:

```text
examples/official/13_file_operations/main.eng
```

## Process Results

Use `run command` when an EngLang workflow must call a visible external tool
such as a validator, converter, simulator, or legacy executable. The command
must bind a `ProcessResult`; this keeps exit status and text output available
for review instead of hiding side effects in a script.

```eng partial
echo_result = run command "cmd"
with {
    args = ["/C", "echo", "eng-process-ok"]
}

log info "external command finished"
```

Current process options:

| Option | Meaning |
|---|---|
| `args` | String array passed as process arguments |
| `cwd` | Working directory, resolved source-relative when relative |
| `env` | Inline object of portable environment variable names and values |
| `tool_version` | Explicit string metadata for the external tool identity |
| `expected_outputs` | Output files that must exist after the process exits |
| `timeout` | Positive duration such as `10 s`, `10 min`, or `1 h` |
| `retry` | Integer retry count from 0 to 5 |
| `allow_failure` | If `true`, non-zero exit code is recorded instead of failing the run |

Rules:

| Rule | Meaning |
|---|---|
| Top-level only | Imported files cannot hide external process execution |
| Binding required | Write `result = run command "tool"` |
| Command is explicit | The command name is a string; arguments live in `with { args = [...] }` |
| Non-zero exits fail by default | Add `allow_failure = true` only when the failure is expected data |
| Reviewable records | `review.json` includes `process_runs[]` |
| Runtime records | Saved runs write `build/result/process_results.json` |

The process result artifact records command, args, env keys, cwd, timeout,
retry policy, attempt count, allow-failure policy, timed-out state, tool
version, exit code, success, stdout/stderr plus hashes, expected output status
and hashes, duration, status, and source line. The binding itself is typed
as `ProcessResult` so IDEs and review tools can show it in variable metadata.

The runnable example is:

```text
examples/official/15_process_result/main.eng
```

## Execution Profiles

Runtime profiles are selected by the CLI, not by source code:

```text
eng.exe run main.eng --profile safe
eng.exe run main.eng --profile normal
eng.exe run main.eng --profile repro
```

Current profile behavior:

| Profile | Behavior |
|---|---|
| `safe` | Rejects explicit workflow `export`, `write`, DB writes, file operations, and `run command` before they execute |
| `normal` | Default profile; allows supported effects and records their artifacts/provenance |
| `repro` | Allows supported effects but records profile diagnostics for environment dependencies, process runs, DB writes, and file mutations |

Profile metadata is written to:

```text
build/result/result.engres
build/result/run_log.json
build/result/output_manifest.json
```

Safe profile still allows internal in-memory checking and report construction.
When `--save-artifacts` is used, EngLang may write its own requested runtime
artifacts, but source-level output effects remain blocked.

## Test Blocks, Assertions, And Golden Checks

`test` blocks keep lightweight workflow verification next to the engineering
calculation. They run after exports, writes, process results, and other runtime
artifacts have been produced. Saved runs write `build/result/test_results.json`;
failed tests make `eng run` fail after the result artifact is available for
inspection.

```eng partial
test "summary values" {
    assert mean_Q == 5.07 kW within 0.01 kW
    assert E_coil > 1 kWh
    golden "summary.csv" matches file("golden/summary.csv")
}
```

Current assertion operators:

```text
==
!=
>
>=
<
<=
```

`within` is allowed only for equality-style checks. The tolerance must be
compatible with the asserted quantity:

```eng partial
assert Q_peak == 5.4 kW within 0.1 kW
```

Golden checks compare a generated artifact under `build/result` against a
source-relative expected file:

```eng partial
golden "summary.csv" matches file("golden/summary.csv")
```

Rules:

| Rule | Meaning |
|---|---|
| Top-level block | `test` belongs at top level |
| Assertion scope | `assert` is valid only inside `test { ... }` |
| Typed operands | Quantity comparisons require compatible dimensions |
| Golden source | Expected files use `file("...")` and resolve source-relative |
| Runtime records | Saved runs write `build/result/test_results.json` |

The runnable example is:

```text
examples/official/16_test_assert_golden/main.eng
```

## Report, Summarize, Show, Plot

`report { ... }` asks for reviewable artifacts. The current report path can
record summaries, plots, system metadata, uncertainty/modeling metadata,
and report/review JSON.

```eng partial
report {
    summarize Q_coil by [mean, time_weighted_mean, max, p95]
    show E_coil
    plot Q_coil over Time
    with {
        unit y = kW
        title = "Coil heat rate"
    }
}
```

`summarize` supports a list of statistic names. Common statistic names:

```text
mean
time_weighted_mean
max
min
median
std
p90
p95
duration_above(5 kW)
```

Plot output under `eng run --save-artifacts` includes:

```text
build/result/plots/plot_spec.json
build/result/plots/plot_manifest.json
build/result/plots/timeseries.svg
build/result/report.html
build/result/run_log.json
build/result/process_results.json
build/result/test_results.json
build/result/output_manifest.json
```

## Systems, Domains, And Internal Tracks

The grammar also has supported system forms and internal domain/component
metadata forms.

Minimal system shape:

```eng partial
system Room {
    state T: AbsoluteTemperature [degC] = 21 degC
    parameter C: HeatCapacity [J/K] = 120000 J/K
    parameter UA: ThermalConductance [W/K] = 150 W/K

    equation {
        C * der(T) eq 500 W - UA * (T - 5 degC)
    }
}
```

The supported measured-vs-simulated simulation path binds a system simulation
to a value and attaches options with `with`:

```eng partial
sim = simulate RoomThermal
with {
    T_out = weather_data.T_out
    timestep = 10 min
    solver = fixed_step
}

rmse_T = rmse measured_data.T_zone vs sim.T_zone
validate rmse_T < 5 K
```

Dynamic inputs are checked as Time-indexed TimeSeries values. The supported
measured-vs-simulated path uses an explicit input contract:

```eng partial
input T_out: TimeSeries[Time] of AbsoluteTemperature [degC]
input solar: TimeSeries[Time] of Irradiance [W/m2]
```

For the same one-state thermal shape, `solver = adaptive_heun` is accepted as a
supported solver option. Internal continuous state-space systems with
shape-checked `der(x) eq A * x + B * u` operators can also use
`adaptive_heun`. These paths keep the report/output TimeGrid fixed by
`timestep` and adapt internal Heun/Euler substeps. A numeric `tolerance` option
may be supplied; broad adaptive equation-system solving remains deferred.

The one-state thermal runner also keeps the earlier scalar input plus
TimeSeries-binding form for narrow compatibility, such as
`input T_out: AbsoluteTemperature` with `simulate ... with { T_out = weather_data.T_out }`.

The supported source-equation ODE workflow extends this surface to systems with
one linked `der(state)` equation per state. Inputs may be scalar values or
Time-indexed TimeSeries bindings, including arbitrary declared input option
names such as `drive`. `solver = fixed_step`, `solver = explicit_euler`,
`solver = rk4`, or `solver = adaptive_heun` runs through the common
SolverInput/SolverResult path. Official examples are
`examples/advanced_solver/20_multi_state_thermal` and
`examples/advanced_solver/34_three_state_source_ode`.

The compiler reports diagnostics for missing dynamic inputs, wrong TimeSeries
quantity, wrong axis, missing `timestep`/`solver`, unsupported solver values,
invalid numeric `tolerance` values, and `timestep` values without duration
units.

Runtime materializes simulated state values as typed TimeSeries such as
`sim.T_zone` for the one-state workflow, `sim.T_air`/`sim.T_wall` for the
thermal source-equation workflow, and `sim.x`/`sim.y`/`sim.z`/`sim.total` for the
three-state non-thermal source-equation workflow. The RMSE result appears in
`computed_metrics`, the validation appears in `validations`, and pairwise
TimeSeries overlap/match status appears in `time_alignments`. Alignment artifacts
also include nominal left/right time steps, irregular-axis flags, and a
`step_status` of `matched`, `mismatch`, or `unavailable`. Explicit
`align <series> with <series>` and `resample <series> to <series>` hooks add
binding, strategy, method, optional resample step, tolerance, and source line to
the same artifact collection. Runtime report specs also include `time_axes`
entries with source column, range, count, nominal step, missing count, and
irregular-axis status per promoted table. RMSE metrics record their
`alignment_reference`, `alignment_status`, and `alignment_step_status` when a
corresponding TimeSeries alignment artifact exists.

The supported component solve surface uses an explicit `solve` binding over the
current component assembly artifact:

```eng partial
fixed_point_result = solve component_graph
with {
    solver = fixed_point
    tolerance = 0.000001
    max_iter = 60
    relaxation = 1
    initial = 4
}
```

This path is intentionally narrow. `solver = fixed_point` is supported for
pivotable linear ResidualGraphs assembled from component connections and simple
component-local equations. The options are plain numeric values; invalid
`tolerance`, `max_iter`, `relaxation`, `initial`, `variable_scale`, and
`variable_scales` values are diagnostics. `variable_scale` applies one positive
update scale to every unknown, while `variable_scales = [...]` supplies
per-unknown positive scales for fixed-point update diagnostics and artifacts.

Static source systems can also be solved explicitly when their equations are
algebraic and square. Use `dense_linear`/`linear` for linear systems,
`fixed_point` when direct or affine single-target `eq` side mappings can be extracted,
and `newton`/`nonlinear_newton` for nonlinear systems:

```eng partial
system StaticNonlinearSourceSystem {
    parameter target: DimensionlessNumber [1] = 4
    state x: DimensionlessNumber = 1
    output y: DimensionlessNumber [1]

    equation {
        x * x eq target
        y eq x + 1
    }
}

source_system_result = solve StaticNonlinearSourceSystem
with {
    solver = newton
    initial = [1, 0]
    tolerance = 0.000000001
    max_iter = 30
}
```

For `solve <SystemName>`, the runtime lowers source equations into the same
residual graph used by component solves. A fixed-point source-system solve can
use equations such as `x eq cos(y)`, `2 * x + 0.1 eq cos(y)`, and `y eq x`, but it still requires a
direct expression mapping rather than arbitrary partition detection. State and
output variables are treated
as static algebraic unknowns, while parameter and scalar input defaults are used
as residual constants. This is not a time simulation path: systems with
`der(...)` equations must use `simulate <SystemName>` when they match a supported
ODE shape.

For source-system Newton solves, `variable_scale = <positive scale>` applies the
same scale to every unknown and `variable_scales = [...]` supplies per-unknown
positive scales. Unit-suffixed literals are converted against the corresponding
unknown units when possible. These options are recorded in solver artifacts as
`user_provided_variable_scales` with `variable_scale_min` and
`variable_scale_max`; omitting them keeps the unit-derived default scale policy.

Simple-linear dynamic component solves use the same binding with dynamic solver
options:

```eng partial
dynamic_room_result = solve component_graph
with {
    solver = dynamic_component_semi_implicit_euler
    timestep = 1 s
    duration = 5 s
    initial = 20 degC
    tolerance = 0.000001
    max_iter = 20
}
```

`dynamic_component_explicit_euler` runs algebraic-free derivative residuals and
`dynamic_component_semi_implicit_euler` solves linear algebraic residuals per
timestep. Both paths emit component solver trajectories and step diagnostics.
They currently require simple linear component-local residual terms;
materialized component parameters may appear as linear coefficients. Nonlinear,
broad constructor-parameterized, adaptive, and production multi-domain component
solves remain outside this source surface. A constrained
Thermal/Fluid[Water] pressure/flow algebraic residual graph is available as a
dense linear solve in `examples/advanced_solver/32_small_thermal_fluid_loop`.

Nonlinear algebraic source residuals can use the same binding with Newton:

```eng partial
nonlinear_result = solve component_graph
with {
    solver = newton
    initial = 1
    tolerance = 0.000000001
    max_iter = 30
}
```

This path evaluates the source residual expressions directly, applies the
component residual scaling policy, calls Newton with finite-difference Jacobian
estimation by default, and records residual history plus largest-residual
artifacts. `jacobian = source_linear_terms` is the only source-level provided
Jacobian hook for component-graph and static source-system Newton solves; it
is valid for residual graphs whose linear terms can be assembled. Broad
symbolic Jacobian declarations are not supported.

Multi-state DAE source residuals use `solver = implicit_euler_dae`:

```eng partial
dae_result = solve component_graph
with {
    solver = implicit_euler_dae
    timestep = 1 s
    duration = 2 s
    initial = 1
    initial_derivative = -2
    initial_algebraic = 2
    tolerance = 0.000000001
    max_iter = 30
}
```

The runtime derives state/algebraic variables from the component assembly,
builds `DaeInput`, runs Newton algebraic initialization by default, uses an
identity mass-matrix fallback unless a dimensionless scalar, diagonal vector, or
dense square `mass_matrix` option is supplied, calls the implicit-Euler DAE
solver, and records state/algebraic trajectories plus step diagnostics. This
path is limited to small scalar component equations using arithmetic over source
residuals.

`mass_matrix = identity` keeps the default. `mass_matrix = 2` broadcasts a
scalar diagonal coefficient, `mass_matrix = [2, 1]` supplies a diagonal vector,
and `mass_matrix = [[2, 0], [0, 1]]` supplies a dense square state mass matrix.
Mass-matrix coefficients are currently dimensionless.

The supported typed-block state-space surface starts with top-level state and
input type blocks:

```eng partial
states RoomState {
    T_air: AbsoluteTemperature [degC]
    T_wall: AbsoluteTemperature [degC]
}

inputs RoomInput {
    T_out: AbsoluteTemperature [degC]
    Q_hvac: HeatRate [W]
}
```

System declarations can then bind those types to solver vectors and operators:

```eng partial
system ContinuousRoomStateSpace {
    state x: StateVector[RoomState] = [22 degC, 20 degC]
    input u: InputVector[RoomInput] = [8 degC, 1800 W]

    operator A: LinearOperator[RoomState -> Derivative[RoomState]] = [[-0.021 1/min, 0.0072 1/min]; [0.0048 1/min, -0.0096 1/min]]
    operator B: LinearOperator[RoomInput -> Derivative[RoomState]] = [[0.0138 1/min, 0.0000012]; [0.0048 1/min, 0.0]]

    equation {
        der(x) eq A * x + B * u
    }
}
```

`examples/advanced_solver/21_state_space_discrete` uses `next(x) eq A * x + B * u`
for the discrete path. `examples/advanced_solver/22_state_space_continuous` uses
`der(x) eq A * x + B * u` with a CSV-backed TimeSeries input and fixed-step
RK4. Both produce `sim.<state>` TimeSeries through the common
SolverInput/SolverResult path.

The legacy state-space metadata syntax remains available for internal fixtures:

```eng partial
system ThermalStateSpaceMetadata {
    state T_zone: AbsoluteTemperature = 22 degC
    input T_out: AbsoluteTemperature = 8 degC
    input Q_internal: HeatRate = 500 W

    states x = [T_zone]
    inputs u = [T_out, Q_internal]
    outputs y = [T_zone]

    A: LinearOperator[StateVector -> Derivative[StateVector]] = [[-0.012 1/min]]
    B: LinearOperator[InputVector -> Derivative[StateVector]] = [[0.012 1/min, 0.001]]

    equation {
        der(x) eq A * x + B * u
    }
}
```

`review.json` records `state_space_vectors` and `linear_operators` for IDE and
review tooling, including per-second `canonical_matrix` values when operator
entries are canonicalizable and `canonical_entries` for nonzero row/column
member pairs. Report artifacts expose the same checked operator metadata.
Vector members must resolve to variables in the same system;
unknown members use `E-STATE-SPACE-VECTOR-MEMBER-001`. Linear operator matrix
rows must match the target vector size, and columns must match the source
vector size; mismatches use `E-STATE-SPACE-OP-SHAPE-001`. Operator artifacts
also include row/column member names, quantity kinds, units, and a compatibility
status. Non-rectangular matrices are reported as shape mismatches. Matrix
entries may be canonical numeric coefficients, or inverse-time coefficients
such as `1/s`, `1/min`, and `1/h` when the target derivative unit is exactly the
source state/input unit per second. Inverse-time display units are canonicalized
to per-second numeric coefficients before runtime/JIT matrix use and report/IDE
inspection. Unsupported coefficient units are diagnosed with
`E-STATE-SPACE-OP-ENTRY-UNIT-001`.
Runtime may materialize state trajectories when shape-checked A/B operators are
available, including supported typed-block discrete/continuous fixed-step
execution, legacy/internal multi-state continuous Euler/RK4 execution,
continuous `adaptive_heun` execution with a fixed output TimeGrid, discrete A/B
execution, and TimeSeries materialization for bound input vector members. This
does not claim broad operator algebra, nonlinear, DAE, discrete adaptive, broad
adaptive, or component-coupled state-space solving.

Domain/component shapes are documented separately in
`docs/internal/component_domain/README.md`. They are useful for metadata, validation, and
IDE inspection, but not yet a general numeric multi-domain solver.

## Review JSON

`eng check --review` writes compiler-owned review metadata. The command/where
with implementation exposes:

```text
syntax_summary.command_styles
syntax_summary.where_blocks
syntax_summary.with_blocks
syntax_summary.tests
command_styles[]
where_blocks[]
with_blocks[]
simulation_requests[]
process_runs[]
tests[]
```

`command_styles[]` records:

| Field | Meaning |
|---|---|
| `verb` | Command verb, such as `integrate` or `mean` |
| `target` | Surface target |
| `clauses` | Parsed command clauses |
| `canonical` | Lowered call string |
| `status` | `lowered`, `ambiguous_target`, or `missing_target` |
| `owner` | Binding name when attached to a binding |
| `line` | Source line |

`where_blocks[]` records owner line, local bindings, inferred quantity kinds,
display units, and local status.

`with_blocks[]` records owner line and accepted/unknown options.
`uncertainty_policies[]` records normalized `with { uncertainty = ... }`
policy metadata, optional sample count, optional seed, and review status.

`simulation_requests[]` records `simulate` bindings, target system, solver,
bound inputs, and compiler-visible time-grid metadata. When `duration` is
declared, the review artifact includes `duration_s` and computed `step_count`;
otherwise TimeSeries-driven simulations use `time_grid.status =
runtime_from_timeseries` and leave duration/step count to runtime artifacts.

`process_runs[]` records explicit external process declarations with binding,
command, and source line. Runtime execution details are written to
`process_results.json` during `eng run`.

`tests[]` records test block names, source lines, assertions, and golden checks.
Runtime pass/fail details are written to `test_results.json` during `eng run`.

This makes syntax policy reviewable without requiring runtime artifact
generation.

## Diagnostics Cheat Sheet

| Code | Meaning | Typical Fix |
|---|---|---|
| `E-CMD-AMBIG-001` | Command target is ambiguous | Parenthesize the target |
| `E-CMD-UNKNOWN-VERB` | Command-style verb is not supported | Use a supported built-in verb or parenthesized function call |
| `E-VALIDATE-BOOL-001` | `validate` target is not a comparison | Write `validate value < threshold` |
| `E-VALIDATE-EXPR-001` | `validate` expression cannot be resolved | Bind the value or fix the name |
| `E-VALIDATE-UNIT-001` | `validate` compares incompatible units | Use a compatible threshold |
| `E-NAME-LOCAL-001` | Where-local used outside owner scope | Move it top-level or use it only in owner |
| `E-WHERE-FWD-001` | Where-local used before definition | Reorder the where bindings |
| `E-WITH-OPTION-001` | Unknown `with` option | Use a supported option key |
| `E-WITH-UNIT-001` | Incompatible display unit | Pick a unit compatible with the owner quantity |
| `E-LOG-LEVEL-001` | Unknown or missing log level | Use `log debug/info/warn/error "..."` |
| `E-PRINT-FMT-003` | Print requested incompatible unit | Fix the print unit |
| `E-PRINT-FMT-004` | Print expression cannot be resolved | Bind the value or fix the name |
| `E-EXPORT-CSV-003` | CSV export expression cannot be resolved | Bind/export a supported scalar |
| `E-EXPORT-CSV-004` | CSV export requested incompatible unit | Fix the export unit |
| `E-WRITE-003` | Write expression cannot be resolved | Bind/write a supported expression |
| `E-WRITE-FMT-003` | `write text` interpolation requested incompatible unit | Fix the interpolation unit |
| `E-WRITE-FMT-004` | `write text` interpolation expression cannot be resolved | Bind the value or fix the placeholder |
| `E-FS-CONFIRM-001` | Move/delete missing confirmation | Add `with { confirm = true }` |
| `E-FS-DELETE-001` | Directory delete missing recursive option | Add `recursive = true` and `confirm = true` |
| `E-PROCESS-BINDING-001` | `run command` has no binding | Write `result = run command "tool"` |
| `E-PROCESS-CMD-001` | `run command` has no command string | Provide the command string and use `args` for arguments |
| `E-TEST-001` | Invalid test block syntax | Use `test "name" { ... }` |
| `E-ASSERT-001` | `assert` is outside a test block | Move it inside `test { ... }` |
| `E-ASSERT-UNIT-001` | Assert operands use incompatible units | Compare compatible quantities |
| `E-ASSERT-TOL-001` | Tolerance used with an unsupported operator | Use `within` with `==` or `!=` |
| `E-GOLDEN-001` | Invalid golden check syntax | Use `golden "artifact" matches file("expected")` |
| `E-UNC-DIRECT-COMPARE` | Uncertain value compared directly | Use `mean(Q)`, `p95(Q)`, or `probability(Q < threshold)` |
| `E-UNC-PROBABILITY-EXPR-INVALID` | `probability(...)` is not an uncertain-threshold comparison | Compare exactly one uncertain value with a compatible threshold |
| `E-UNC-PERCENTILE-UNIT-MISMATCH` | Uncertainty percentile threshold has incompatible units | Use a threshold with the percentile quantity's dimension |
| `E-UNC-TS-STD-001` | TimeSeries `sensor_std` metadata is invalid | Attach a non-negative unitful value to a compatible TimeSeries |
| `W-QTY-AMBIG-001` | Unit maps to multiple quantity kinds | Add an explicit declaration |

## Common Recipes

### Heat Rate From CSV

```eng partial
sensor = promote csv args.input as SensorData
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

### Energy From Heat Rate

```eng partial
E_coil = integrate Q_coil over Time
```

### Local Source For One Integration

```eng partial
E_coil = integrate Q_for_energy over Time
where {
    Q_for_energy = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
}
with {
    method = trapezoidal
}
```

### Print And Export A Summary

```eng partial
mean_Q = mean Q_coil over Time
peak_Q = max Q_coil over Time

print "Q mean = {mean_Q: .2 kW}"
print "Q peak = {peak_Q: .2 kW}"
log info "summary values computed"
log warn "review peak load if this is a design case"

export summary to csv "summary.csv" {
    mean_Q as kW with ".2"
    peak_Q as kW with ".2"
}
with {
    overwrite = true
}
```

### Write A Small Output

```eng partial
write text "outputs/run_note.txt", "finished"
with {
    overwrite = true
}
```

### Log A Runtime Message

```eng partial
log info "case started"
log debug "mean Q = {mean_Q: .2 kW}"
log warn "manual review recommended"
```

### Copy And Delete Generated Outputs

```eng partial
copy file("data/template.txt") to "outputs/template.txt"

delete "outputs/template.txt"
with {
    confirm = true
}
```

### Run An External Process

```eng partial
process_result = run command "cmd"
with {
    args = ["/C", "echo", "ok"]
    expected_outputs = ["outputs/tool-output.txt"]
}

log info "process result captured"
```

### Check A Summary Artifact

```eng partial
test "summary artifact" {
    assert mean_Q > 0 kW
    assert E_coil == 1.26 kWh within 0.02 kWh
    golden "summary.csv" matches file("golden/summary.csv")
}
```

### Imported Function

```eng partial
use "thermal.eng"

UA_wall = 150 W/K
dT_wall = 8 K
Q_wall = heat_loss(UA_wall, dT_wall)
print "Q wall = {Q_wall: .2 kW}"
```

## What Is Deferred

The current guide intentionally does not promise:

| Deferred Area | Current Position |
|---|---|
| General command syntax for all functions | User/general calls stay parenthesized |
| Project-wide display unit policy block | Deferred; `with` is local owner options |
| First-class Summary object model | Deferred; explicit CSV summary export exists |
| Arbitrary TimeSeries formulas | Limited beyond official heat-rate kernel path |
| Broad table/TimeSeries CSV export | Deferred |
| Broad filesystem mutation outside generated outputs | Deferred |
| Full process sandboxing | Explicit process records and profile basics exist; sandbox isolation is deferred |
| Project-wide test discovery/runner | Local source-file test blocks exist; workspace discovery is deferred |
| Full package/module system | File imports and declared metadata only |
| General nonlinear/DAE/behavior/broad adaptive/multi-domain solving | Deferred beyond supported one-state thermal fixed/adaptive path, supported two-state source-equation fixed-step path, narrow component residual Newton/implicit-Euler DAE smokes, narrow unitful temperature explicit-Euler source behavior RHS smokes, constrained Thermal/Fluid[Water] pressure/flow algebraic residual smoke, and internal fixed-step/continuous `adaptive_heun` state-space paths |
| Full artifact schema evolution policy | Stable-core schemas exist; broader future-track schemas may grow |

## Authoring Checklist

Before treating a file as a good EngLang example, check:

1. Does it use top-level workflow statements instead of `script main`?
2. Does it use root `args { ... }` for inputs?
3. Are public quantities explicit where unit inference would be ambiguous?
4. Are general function calls parenthesized?
5. Are command-style forms limited to built-in workflow verbs?
6. Are complex command targets parenthesized?
7. Are `where` locals used only by their owner expression?
8. Are `with` option keys supported and units compatible?
9. Does `print` stay lightweight and debugging-oriented?
10. Do structured runtime messages use `log debug/info/warn/error`?
11. Are durable outputs written with `export`, `plot`, and `report`?
12. Are file operations explicit, confirmed where destructive, and kept inside the output boundary?
13. Are external commands explicit `run command` statements with a bound `ProcessResult`?
14. Are assertions and golden checks grouped under named `test` blocks?
15. Does `eng.exe check --review` show useful metadata?
16. Does `eng.exe run --save-artifacts` produce expected artifacts?

## Official Example Walkthrough

`examples/official/09_command_where_with/main.eng` is the recommended example
for this grammar policy.

It demonstrates:

| Line Family | Purpose |
|---|---|
| `schema SensorData` | Typed CSV boundary |
| `args { input: CsvFile = ... }` | User-provided CSV input |
| `sensor = promote csv ...` | Runtime data promotion |
| `Q_coil = ...` | HeatRate TimeSeries binding |
| `E_coil = integrate Q_for_energy over Time` | Command-style integration |
| `where { Q_for_energy = ... }` | Owner-local source calculation |
| `with { method = trapezoidal }` | Owner option metadata |
| `mean_Q = mean Q_coil over Time` | Command-style statistic |
| `print ...` | Direct CLI/debug output |
| `log ...` | Structured runtime message |
| `export summary to csv` | Durable scalar CSV artifact |
| `test ...` | Runtime assertions and golden artifact comparison |
| `report { summarize ... plot ... with ... }` | Review/report/plot output |

Run it:

```text
eng.exe run examples/official/09_command_where_with/main.eng --save-artifacts
```

Expected user-facing output includes lines similar to:

```text
Q mean = 5.07 kW
Q peak = 5.42 kW
E total = 1.26 kWh
```

Expected artifacts include:

```text
build/result/summary.csv
build/result/output_manifest.json
build/result/run_log.json
build/result/process_results.json
build/result/test_results.json
build/result/review.json
build/result/report.html
build/result/plots/plot_spec.json
build/result/plots/timeseries.svg
```
