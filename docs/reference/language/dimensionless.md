# Dimensionless Policy

Dimensionless is a formal quantity category in EngLang.

## Addition and Subtraction

Dimensionless literals cannot be implicitly added to or subtracted from physical quantities.

Not allowed:

```eng error
X = 1 m + 20
Q = 1 + 2 kW
Q_expected: HeatRate [kW] = 2 kW - 1
T = 24 degC + 1
```

Diagnostics:

```text
E-DIM-ADD-001
E-DIM-ADD-002
E-DIM-ADD-003
E-DIM-ADD-004
```

Allowed:

```eng
X = 1 m + 20 cm
Q = 1 kW + 2 kW
T = 24 degC + 1 K
```

Expected type does not add units automatically. This is still invalid:

```eng error
Q: HeatRate [kW] = 2 kW - 1
```

The `1` must carry an explicit unit if it means `1 kW`.

## Multiplication and Division

Dimensionless scaling is allowed.

```eng partial
Q_loss = 0.85 * Q_nominal
L2 = 2 * L
L3 = L / 2
eta = P_out / P_in
```

## Ratios And Percentages

`Ratio` uses canonical unit `1`. Percentage literals may be attached or spaced;
both forms preserve `%` as the source unit. An explicit `Ratio [%]` target keeps
`%` for display, while an inferred ratio uses canonical display unit `1`.

```eng partial
efficiency = 25%
reserve_margin = 15 %
normalized_efficiency = 0.25 1
target: Ratio [%] = 75%
```

`25%` converts to `0.25 1`, and `0.25 1` converts to `25 %` when `%` is the
requested display unit. A bare `1` remains an ordinary dimensionless number;
`[1]` and the suffix in `0.25 1` are unit contexts.

## Scalar Math

`sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`, and `atan` require
one dimensionless argument and return `DimensionlessNumber [1]`. The compiler
types direct, nested, and arithmetic results, and the native runtime evaluates
the same expressions for scalar result artifacts.

```eng
ratio = 0.25
root = sqrt(ratio)
result = exp(ln(root)) + sin(0)
```

Unitful and unresolved arguments, wrong arity, and a dimensionless result used
for an incompatible annotated declaration are compiler errors. The editor
underlines the failing function name or argument rather than only coloring the
call as a built-in.

## Ambiguous Quantity

Some units map to multiple quantity kinds.

```eng
power = 10 kW
```

This produces:

```text
W-QTY-AMBIG-001
```

The diagnostic lists candidate quantity kinds such as `HeatRate`, `ElectricPower`, and `MechanicalPower`.
