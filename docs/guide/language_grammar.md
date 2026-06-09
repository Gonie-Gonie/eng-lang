# EngLang Language Grammar Guide

This guide is the practical grammar reference for the current EngLang preview.
It is written for someone who wants to open an `.eng` file, understand the
shape of the language, and write a small engineering workflow without reading
the compiler source.

EngLang is still preview software. The behavior below is documented and tested
for the current public line, but it is not yet a stable language contract.

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
| Bytecode | Native runtime seed for the top-level workflow |
| Runtime data | Tables, TimeSeries, integrations, statistics, outputs |
| Report artifacts | `result.engres`, `review.json`, `report_spec.json`, `run_log.json`, PlotSpec, SVG, HTML |

Use these commands from the repository or portable package:

```text
eng.exe check examples/official/09_command_where_with/main.eng --review
eng.exe run examples/official/09_command_where_with/main.eng --save-artifacts
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
| Package import seed | `import package.name` | Declared metadata path, not full package manager |
| Args block | `args { input: CsvFile = file("data.csv") }` | Root CLI argument schema |
| Const declaration | `const cp: SpecificHeatCapacity = 4180 J/kg/K` | Importable when pure |
| Explicit declaration | `E: Energy [J] = 3600 J` | Public type and display unit boundary |
| Fast binding | `Q = 10 kW` | Inferred declaration |
| Schema | `schema SensorData { ... }` | CSV/data boundary |
| Function | `fn heat_loss(...) -> HeatRate [W] { ... }` | Typed scalar preview |
| System | `system Room { ... }` | Minimal equation/system preview |
| Domain/component | `domain Fluid { ... }` | Experimental metadata track |
| Print | `print "Q = {Q: .2 kW}"` | Debug/CLI output |
| Log | `log warn "Q is high"` | Structured runtime message |
| CSV export | `export summary to csv "summary.csv" { ... }` | Durable scalar artifact |
| Write output | `write text "note.txt", note` | Explicit generated output |
| File operation | `copy file("note.txt") to "outputs/note.txt"` | Explicit output-area file mutation |
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
    case_name: String = "preview"
    enabled: Bool = true
    count: Count = 3
    gain: Float = 1.0
    window: Duration = 10 min
}
```

Supported preview argument types include:

| Type | Example Default | CLI Shape |
|---|---|---|
| `String` | `"preview"` | `--case_name demo` |
| `Path` / `FilePath` / `CsvFile` | `file("data/sensor.csv")` | `--input data/other.csv` |
| `DirectoryPath` | `dir("runs")` | `--output runs/case1` |
| `Bool` | `true` | `--enabled true` |
| `Int` / `Integer` / `Count` | `3` | `--count 12` |
| `Float` / `Number` | `1.0` | `--gain 1.25` |
| `Duration` | `10 min` | `--window 30 s` |

Runtime records the final bound value and whether it came from the default or
CLI override.

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

The current preview resolves relative paths against the source file directory.
For `examples/official/10_path_policy/main.eng`, the review/result/report-spec
artifacts contain an `environment_dependencies` entry for `exists args.input`.

## Read-Only I/O

Use read-only I/O when a workflow needs small UTF-8 text/config companion files
in addition to typed engineering data. The current preview returns raw strings;
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
| Source hashes are recorded | `review.json`, `result.engres`, and `report_spec.json` include provenance |
| Hidden imported reads are rejected | Importable const/function files must not hide runtime I/O |

The runnable example is:

```text
examples/official/11_read_only_io/main.eng
```

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
| One return expression | Preview functions use one `return ...` |
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

Supported command-style verbs in the current preview:

| Verb | Typical Use | Canonical Shape |
|---|---|---|
| `integrate` | HeatRate over Time to Energy | `integrate(Q, over=Time)` |
| `mean` | TimeSeries mean | `mean(Q, axis=Time)` |
| `max` | TimeSeries maximum | `max(Q, axis=Time)` |
| `min` | TimeSeries minimum | `min(Q, axis=Time)` |
| `duration` | Duration style metadata seed | `duration(T, above=...)` |
| `plot` | Report plot request | `plot(Q, over=Time)` metadata |
| `show` | Report/display request seed | `show(value)` metadata |
| `validate` | Validation request seed | `validate(target)` metadata |

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
| `output` | Artifact/output choice |
| `overwrite` | Allow replacing changed generated output content |
| `confirm` | Required confirmation for destructive file operations |
| `recursive` | Required for directory delete operations |

Unknown options are rejected with `E-WITH-OPTION-001`.

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

This keeps the plot request readable while keeping display details grouped
below it.

The older block-style plot option form is also still used in existing examples:

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

The current preview supports scalar values, statistics, integration results,
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

Rules:

| Rule | Meaning |
|---|---|
| Top-level only | Hidden imported writes are not part of the preview |
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

## File Operations

Use file operations when a workflow needs a small, explicit filesystem mutation
that is still reviewable. The current preview keeps the mutation boundary
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

## Report, Summarize, Show, Plot

`report { ... }` asks for reviewable artifacts. The current report path can
record summaries, plots, system metadata, uncertainty/modeling preview metadata,
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
build/result/output_manifest.json
```

## Systems, Domains, And Experimental Tracks

The grammar also has preview/experimental surfaces for systems and
domain/component metadata.

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

Domain/component shapes are documented separately in
`docs/guide/domain_component.md`. They are useful for metadata, validation, and
IDE inspection, but not yet a general numeric multi-domain solver.

## Review JSON

`eng check --review` writes compiler-owned review metadata. The command/where
with implementation exposes:

```text
syntax_summary.command_styles
syntax_summary.where_blocks
syntax_summary.with_blocks
command_styles[]
where_blocks[]
with_blocks[]
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

This makes syntax policy reviewable without requiring runtime artifact
generation.

## Diagnostics Cheat Sheet

| Code | Meaning | Typical Fix |
|---|---|---|
| `E-CMD-AMBIG-001` | Command target is ambiguous | Parenthesize the target |
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
| `E-FS-CONFIRM-001` | Move/delete missing confirmation | Add `with { confirm = true }` |
| `E-FS-DELETE-001` | Directory delete missing recursive option | Add `recursive = true` and `confirm = true` |
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
| Full package/module system | File imports and metadata seeds only |
| General nonlinear/multi-state solving | Deferred beyond preview system path |
| Stable artifact schemas | Preview versioned artifacts only |

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
13. Does `eng.exe check --review` show useful metadata?
14. Does `eng.exe run --save-artifacts` produce expected artifacts?

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
build/result/review.json
build/result/report.html
build/result/plots/plot_spec.json
build/result/plots/timeseries.svg
```
