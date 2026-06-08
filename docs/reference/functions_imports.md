# Functions And File Imports

EngLang preview functions are pure quantity-aware helpers. They are intended for
reusable scalar engineering relationships, not stateful systems or artifact
generation.

## Function Syntax

```eng
fn heat_loss(UA: Conductance [W/K], dT: TemperatureDelta [K]) -> HeatRate [W] {
    return UA * dT
}
```

Rules:

- Parameters require explicit quantity annotations.
- Parameter display units are optional but recommended at public boundaries.
- Return quantity is explicit and unit-checked against the return expression.
- The preview function body supports one `return` expression.
- Function-local bindings are not promoted into the global variable table.

## File Imports

```text
use "thermal.eng"
```

Rules:

- File imports are resolved relative to the importing source file.
- Imported files contribute function definitions only in this preview.
- Imported scripts are not registered as entry points and are not executed.
- Import cycles and unreadable paths are compile diagnostics.
- Source files should be UTF-8 encoded.

## Example

```text
use "thermal.eng"

script main(args: Args) -> Report {
    UA_wall = 150 W/K
    dT_wall = 8 K
    Q_wall = heat_loss(UA_wall, dT_wall)

    print "Q wall = {Q_wall: .2 kW}"

    export summary to csv "summary.csv" {
        Q_wall as kW with ".2"
    }
}
```
