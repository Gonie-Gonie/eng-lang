# Class Object Guide

This guide documents the current metadata-first class/domain-object preview.
It is for typed engineering library objects such as buildings, zones, and
constructions. It is not a runtime object-dispatch system yet.

## Class Declarations

```text
class Construction {
    name: String
    u_value: Conductance [W/K]
    thickness: Length [m] = 0.2 m
    validate {
        name != ""
        u_value > 0 W/K
    }
}
```

Fields have a name, type, optional display unit, and optional default value.
Fields without defaults are required in object literals.

Supported field type categories in the preview:

- built-in scalar/path types such as `String`, `Bool`, `Count`, and `Float`;
- built-in quantity types such as `Length`, `Conductance`, and `HeatCapacity`;
- another declared class name for nested object references.

## Class Validations

```text
class Zone {
    name: String
    capacity: HeatCapacity [J/K]
    occupancy: Count = 1
    validate {
        name != ""
        capacity > 0 J/K
        occupancy >= 0
    }
}
```

Class validation blocks are metadata-first object invariant checks. In the
current preview they support simple field comparisons such as `field > 0 unit`,
`field <= 1`, and `field != ""`. The check path evaluates object literal values
and class defaults when possible, then records `pass`, `fail`, or `unresolved`
results in the object summary.

## Object Literals

```text
wall = Construction {
    name = "south_wall"
    u_value = 120 W/K
}
```

Object literals are recorded as typed metadata bindings with type
`Object[ClassName]`. They are reviewable artifacts, not mutable runtime
objects. A field value may be a string, bool, numeric literal with a compatible
unit, typed binding, function result, or another class object.

Nested objects use declared class types:

```text
class Zone {
    name: String
    capacity: HeatCapacity [J/K]
}

class Building {
    name: String
    zone: Zone
    envelope: Construction
}

office = Zone {
    name = "office"
    capacity = 120000 J/K
}

building = Building {
    name = "Lab"
    zone = office
    envelope = wall
}
```

## Field Access

```text
wall_u = wall.u_value
```

Field access is type-checked from class/object metadata. The example above is
typed as `Conductance [W/K]`, appears in the variable table, and is available to
review/report/IDE metadata consumers.

## Diagnostics

| Diagnostic | Trigger |
|---|---|
| `E-CLASS-OBJECT-001` | Object literal references an unknown class. |
| `E-CLASS-OBJECT-002` | Object field is not attached to an object literal. |
| `E-CLASS-FIELD-MISSING-001` | Object omits a required class field. |
| `E-CLASS-FIELD-UNKNOWN-001` | Object sets a field not declared by its class. |
| `E-CLASS-FIELD-TYPE-001` | Field value has an incompatible type or quantity. |
| `E-CLASS-FIELD-TYPE-002` | Class field uses an unknown type. |
| `E-CLASS-VALIDATION-001` | Class validation rule is not a supported comparison. |
| `E-CLASS-VALIDATION-002` | Object literal fails a class validation rule. |

Diagnostic fixtures live under `examples/05_error_messages/`:

- `class_missing_field.eng`
- `class_unknown_field.eng`
- `class_field_type_mismatch.eng`
- `class_validation_fail.eng`

## Artifact Surface

`eng check --review` writes class and object information to `review.json`:

```bat
target\debug\eng.exe check examples\official\19_class_object_preview\main.eng --review
```

`eng run --save-artifacts` carries the same metadata into `report_spec.json`
and `report.html`:

```bat
target\debug\eng.exe run examples\official\19_class_object_preview\main.eng --save-artifacts
```

The preview artifact sections are:

```text
class_summary
object_summary
```

`class_summary` includes validation rules. `object_summary` includes
per-object validation results with left/right values, unit, and status.

The native IDE artifact outline and LSP snapshot path expose the same sections,
plus class/object hover and completion metadata.

## Support Boundary

Current:

- parser and AST support for `class` declarations;
- typed fields with default values;
- object literals;
- nested object references;
- field access type metadata;
- simple class validation blocks with object-level pass/fail artifacts;
- missing, unknown, and incompatible field diagnostics;
- review/report artifact sections;
- official example and CLI smoke coverage;
- IDE keyword/snippet completion and artifact-outline visibility;
- LSP class/object hover and completion metadata.

Deferred:

- methods;
- `self` access;
- copy-with syntax;
- mutation;
- runtime object dispatch/lowering;
- inheritance;
- class-contained `port`/`connect` declarations.
