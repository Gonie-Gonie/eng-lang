# Top-Level Execution, Args, Imports, And Const

EngLang files run naturally as top-level workflows. `script main` and other
`script` blocks are not supported as execution roots.

## Execution Model

When a file is run directly:

1. Static `use` / `import` file imports are resolved first.
2. The root file `args { ... }` block is evaluated and CLI overrides are bound.
3. Declarations and executable statements are processed in source order.
4. `print`, `report`, `plot`, and `export` statements create explicit output.
5. Any `script` block is diagnosed with `E-SCRIPT-001`; move the body to
   top-level statements and use `args { ... }` for CLI arguments.

```eng
args {
    case_name: String = "baseline"
}

const eta_nominal: Ratio = 0.864

Q = 5.432 kW
eta = eta_nominal

print "case = {args.case_name}"
print "Q = {Q: .2 kW}"
print "eta = {eta: .3}"
```

The recommended file order is:

```text
use/import
args
const/fn/schema/system/domain/component declarations
executable body
report/plot/export
```

This order is style guidance. Declarations and executable statements may be
mixed so interactive work can be saved into a file without a wrapper.

## Args

`args { ... }` declares the root execution arguments. This is the only
supported args declaration syntax.

```eng
const default_input: CsvFile = file("sensor.csv")

fn default_output_dir() -> DirectoryPath = dir("build/result")

args {
    input: CsvFile = default_input
    output: DirectoryPath = default_output_dir()
}

Q = 5 kW
```

Rules:

- CLI `--<field> <value>` overrides the default.
- `file("...")`, `dir("...")`, string literals, importable `const`, and
  deterministic zero-arg pure functions can provide defaults.
- Environment/time/current-directory defaults are allowed in supported workflows but
  reported with `W-ARGS-RUNTIME-DEFAULT-001`.
- Side-effecting defaults such as `download(...)` are rejected with
  `E-ARGS-SIDE-EFFECT-001`.
- Imported module args are ignored and are not bound into the root context.

## Const

Top-level `name = expr` is always an executable local binding. It is not
importable. Shared module values must use explicit `const`.

```eng
const UA_wall_default: Conductance [W/K] = 150 W/K

fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {
    UA_local = UA
    dT_local = dT
    return UA_local * dT_local
}
```

Rules:

- `const` is immutable and importable when declared at top level.
- `const` can use literals, pure expressions, importable const values, and
  deterministic pure helpers.
- `const` must not depend on `args`; this is `E-CONST-ARGS-001`.
- Side-effecting const expressions are rejected with
  `E-CONST-SIDE-EFFECT-001`.
- Runtime-dependent const expressions are warned with `W-CONST-RUNTIME-001`.

## Imports

```eng partial
use "thermal.eng"

UA_wall = UA_wall_default
dT_wall = 8 K
Q_wall = heat_loss(UA_wall, dT_wall)
```

File imports are static and relative to the importing source file.

Imported:

- top-level `const`;
- `fn` definitions and their function-local bindings;
- `schema`, `system`, `domain`, and `component` declarations.

Not imported or executed:

- imported module `args`;
- top-level `name = expr`;
- `promote`, `print`, `report`, `plot`, `export`, and other executable body
  statements;
- imported `script` blocks.

Dynamic import paths are rejected with `E-IMPORT-DYNAMIC-001`, including
`use args.input`, path-helper expressions such as `use join(args.dir, "...")`,
and template-style file strings containing runtime placeholders.
References to top-level imported-module `name = expr` locals are diagnosed as
`E-IMPORT-SYMBOL-001`; use `const name: Type = expr` instead.

## Function Locals

Function-local bindings are visible only inside the function body. They are
compiled as part of the function and are available when the function is called,
but they are not importable symbols and do not appear as root runtime locals.
