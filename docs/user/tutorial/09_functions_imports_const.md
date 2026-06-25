# 09 Functions, Imports, And Const

## Goal

Organize reusable calculations while keeping execution behavior predictable.

## What You Will Build

Use the official functions and imports example:

```bat
eng.exe run examples/official/07_functions_imports/main.eng --out build/runs/functions
```

Representative source patterns:

```eng partial
const cp_water = 4180 J/kg/K

fn heat_rate(m_dot: MassFlowRate, delta_T: TemperatureDifference) {
    return m_dot * cp_water * delta_T
}
```

## Expected Artifacts

The run should produce normal result and review artifacts. Imported files should
provide declarations without unexpectedly executing root workflow statements.

## Explanation

EngLang files run naturally as top-level workflows. Imports are for reusable
declarations and helpers. Keep side effects in the root workflow unless a
reference document explicitly marks another pattern as supported.

## Common Mistakes

- Hiding workflow execution in an imported helper file.
- Using functions to erase units or schema meaning.
- Reusing constants without checking their quantity and display unit.

## What To Inspect

Inspect symbol and calculation metadata in the IDE. For details, read
docs/reference/functions_imports.md and
docs/reference/top_level_execution_policy.md.
