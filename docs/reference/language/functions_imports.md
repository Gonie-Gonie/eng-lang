# Functions And File Imports

EngLang functions are pure quantity-aware helpers. They are intended for
reusable scalar engineering relationships, not stateful systems or artifact
generation.

## Function Syntax

```eng
const UA_wall_default: Conductance [W/K] = 150 W/K

fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {
    UA_local = UA
    dT_local = dT
    return UA_local * dT_local
}
```

Rules:

- Parameters require explicit quantity annotations.
- Parameter display units are optional but recommended at public boundaries.
- Return quantity is explicit and unit-checked against the return expression.
- The current function body supports ordered local `name = expr` bindings plus
  one `return` expression.
- Function-local bindings are part of the function body, not importable module
  symbols.
- Functions are pure. Side-effecting statements and expressions such as
  `print`, `write`, `export`, `run command`, `read text/json/toml`, and
  `promote` are rejected inside function bodies with `E-FN-SIDE-EFFECT-001`.
- Top-level `const` declarations are importable and can be used by functions
  or root workflows.

VS Code signature help uses the compiler's resolved parameter and return
metadata for local and statically imported functions. If a user function has
the same name as a built-in, the user function wins at that call site.

Built-in calls use the same protocol, but their signatures come from validated
`stdlib/eng/modules.toml` symbol contracts plus compiler-owned scalar math and
numeric-percentile catalogs. The catalog distinguishes required and optional
parameters, supports overloads such as `measured(...)`, and preserves declared
return units such as `Duration [s]`. Command-style workflow statements do not
pretend to be ordinary functions.

## Dimensionless Scalar Math

The core math calls `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`,
and `atan` each accept one `DimensionlessNumber [1]` expression and return
`DimensionlessNumber [1]`. They work in direct, nested, and scalar-arithmetic
bindings and are evaluated by the native runtime:

```eng
ratio_input = 0.25
root = sqrt(ratio_input)
response = exp(ln(root))
combined = sin(response) + cos(0)
```

Passing a physical quantity is not an implicit conversion:

```eng error
length = 2 m
invalid = sqrt(length)
```

The compiler reports exact call/argument ranges with `E-MATH-CALL-001`,
`E-MATH-ARG-001`, `E-MATH-DIM-001`, or `E-MATH-RESULT-001`. A user-defined
function with the same name still wins at its call sites.

## File Imports

```text
use "thermal.eng"
```

Rules:

- File imports are resolved relative to the importing source file.
- Imported files contribute top-level `const`, `fn`, `schema`, `system`,
  `domain`, and `component` declarations.
- Imported module `args` blocks and top-level `name = expr` executable locals
  are not imported.
- Imported `script` blocks are not executable roots and are not executed.
- Dynamic import paths are rejected; imports must be static strings. This
  includes args-based targets, path-helper expressions, and template-style file
  strings containing runtime placeholders.
- Import cycles and unreadable paths are compile diagnostics.
- Source files should be UTF-8 encoded.

## Stdlib Module Imports

```text
use eng.net
import eng.table
```

`eng.*` imports are resolved against
[`stdlib/eng/modules.toml`](../../../stdlib/eng/modules.toml), not the file
import resolver. Supported and native workflow modules are accepted as module
boundary declarations for editor navigation and review metadata.

Planned or internal modules produce warnings so the editor can underline the
boundary without treating it as a missing file. Unknown `eng.*` module names are
errors with a linkable registry fix path.

## Example

```text
use "thermal.eng"

args {
    label: String = "wall"
}

UA_wall = UA_wall_default
dT_wall = 8 K
Q_wall = heat_loss(UA_wall, dT_wall)

print "Q {args.label} = {Q_wall: .2 kW}"

export summary to csv "summary.csv" {
    Q_wall as kW with ".2"
}
```

See [top_level_execution_policy.md](top_level_execution_policy.md) for the
full top-level args/import/const execution policy.
