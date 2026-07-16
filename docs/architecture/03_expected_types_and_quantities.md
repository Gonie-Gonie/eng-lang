# Expected Types And Quantity Completions

The expected-types layer records the quantity, unit, and type expectations that
compiler diagnostics, review artifacts, LSP hover, completion, and native IDE
inspection all share.

Current semantic review records include:

```text
ExpectedType
TypeInfo
UnitDerivation
HoverHint
```

## Expected Types

An expected type records the quantity and display unit the compiler expects for a
binding or expression boundary. The most visible source is an explicit
annotation:

```eng
Q: HeatRate [kW] = 1 kW + 2 kW
```

Review data records the binding, quantity kind, display unit, source label, and
source line so diagnostics and editor hovers can point back to the same code.

```json
{
  "name": "Q",
  "quantity_kind": "HeatRate",
  "display_unit": "kW",
  "source": "explicit_annotation",
  "line": 1
}
```

Expected types are also used to keep dimensionless literals from silently taking
on physical units. This remains an error because the right-hand side subtracts a
dimensionless value from a heat-rate expression:

```eng error
Q: HeatRate [kW] = 2 kW - 1
```

## Quantity Completion Catalog

`quantities.rs` is the compiler-owned quantity completion catalog used by
ambiguous-unit diagnostics, inferred declaration metadata, generated editor
metadata, LSP completions, VS Code fallback metadata, and native IDE completion
items.

Public quantity completions include:

```text
AbsoluteTemperature [K]
TemperatureDelta [K]
Length [m]
Area [m2]
Volume [m3]
Conductance [W/K]
HeatCapacity [J/K]
SpecificHeat [J/kg/K]
HeatRate [W]
ElectricPower [W]
MechanicalPower [W]
Energy [J]
Duration [s]
Irradiance [W/m2]
PeopleDensity [person/m2]
Pressure [Pa]
MassFlowRate [kg/s]
Ratio [1]
DimensionlessNumber [1]
ReynoldsNumber [1]
```

The compiler-owned unit registry records both `1` and `%`. Percentage literals
such as `25%` and `25 %` infer `Ratio`, retain `%` as their source unit, and use
`1` as the canonical unit with a `0.01` scale.

The catalog is used to:

```text
- find candidate quantity kinds for a unit
- generate ambiguous quantity warnings and help text
- choose conservative inferred declaration types
- populate LSP, VS Code, and native IDE completion items
```

## Ambiguous Quantities

Some units map to several engineering meanings. `kW` can be heat rate,
electric power, or mechanical power:

```eng
power = 10 kW
```

The compiler keeps this ambiguous unless the name or an explicit annotation gives
enough evidence:

```text
W-QTY-AMBIG-001:
  `power` has unit kW, but quantity kind is ambiguous.

Candidate quantity kinds:
  HeatRate, ElectricPower, MechanicalPower
```

Name-based hints are intentionally conservative:

```eng
Q_cooling = 10 kW   // HeatRate
P_fan = 10 kW       // ElectricPower
shaft_power = 10 kW // MechanicalPower
power = 10 kW       // warning remains ambiguous
```

## Dimensionless Operations

Addition and subtraction checks reject accidental mixing between physical and
dimensionless values:

```eng error
X = 1 m + 20
Q = 1 + 2 kW
Q_expected: HeatRate [kW] = 2 kW - 1
T = 24 degC + 1
```

Compatible expressions stay valid:

```eng partial
L = 1 m + 20 cm
Q = 1 kW + 2 kW
scale = 0.85
Q_loss = 0.85 * Q_nominal
```

## Hover And Type Records

Hover hints expose inferred or explicit binding information to the LSP, VS Code,
and native IDE:

```eng
L = 1 m + 20 cm
```

```text
L
  inferred as Length [m]
  quick fix: Expand declaration
```

`TypeInfo` records the semantic type summary per binding:

```text
name
quantity_kind
display_unit
canonical_unit
dimension
source
line
```

`UnitDerivation` records how a source expression reached its display and
canonical unit:

```text
name
expression
source_unit
display_unit
canonical_unit
quantity_kind
steps
line
```

For example, `L = 1 m + 20 cm` records a length binding with meter display and
canonical units plus derivation steps for the conversion.

## Boundaries

Expected-type records and quantity completions are intentionally conservative.
When the compiler cannot prove a quantity kind, it should warn or ask for an
explicit annotation instead of guessing. Broader propagation through every
possible table, object, or workflow expression should be documented only when the
compiler and runtime artifacts prove that path.
