# Dimensionless Policy

Dimensionless is a formal quantity category in EngLang.

## Addition and Subtraction

Dimensionless literals cannot be implicitly added to or subtracted from physical quantities.

Not allowed:

```eng
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

```eng
Q: HeatRate [kW] = 2 kW - 1
```

The `1` must carry an explicit unit if it means `1 kW`.

## Multiplication and Division

Dimensionless scaling is allowed.

```eng
Q_loss = 0.85 * Q_nominal
L2 = 2 * L
L3 = L / 2
eta = P_out / P_in
```

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

