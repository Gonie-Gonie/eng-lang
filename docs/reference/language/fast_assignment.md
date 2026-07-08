# Fast Assignment

EngLang uses `=` for local binding and assignment. A new name creates a local
binding; an existing name is assigned after compatibility checks.

## Rule

```text
If the name is new in the current local scope:
  name = expr creates a local binding and infers semantic type information.

If the name already exists:
  name = expr assigns after compatibility checks.
```

The `:=` operator is not part of EngLang syntax.

```eng error
Q := 10 kW
```

Diagnostic:

```text
E-SYNTAX-DECL-001
```

## Local Convenience

Allowed:

```eng
L = 1 m + 20 cm
E = 1 kWh + 500 Wh
eta = 0.85
Q_cooling = 10 kW
```

The compiler records the inferred declaration in review artifacts.

```text
name
quantity kind
display unit
expression
type info
unit derivation
hover hint
```

Top-level `name = expr` is still an executable local binding. It runs when the
file is the root workflow, but it is not importable from another module. Use
`const name: Type = expr` for reusable module values.

## Public Boundaries

Public boundaries require explicit type annotations.

```eng
schema SensorData {
    T_supply: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}
```

Not allowed:

```eng error
schema SensorData {
    T_supply = 24 degC
}
```

Diagnostic:

```text
E-PUBLIC-ANNOTATION-001
```

## Review Output

Review artifacts include the inferred declaration, type information, unit
derivation, and hover hint for each local binding. IDE, LSP hover, and quick-fix
features consume that metadata directly instead of re-inferring the assignment
rules in editor code.
