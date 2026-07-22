# Class Object Guide

This guide documents the current supported class/domain-object authoring scope.
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

Supported field type categories:

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
current supported scope includes simple field comparisons such as `field > 0 unit`,
`field <= 1`, and `field != ""`. The check path evaluates object literal values
and class defaults when possible, then records `pass`, `fail`, or `unresolved`
results in the object summary.

## Methods And Copy-With

```text
class Building {
    name: String
    method summary() -> String = self.name
}

building_summary = building.summary()
```

The current method support covers zero-argument metadata methods that return
direct `self.<field>` values. Method calls in expressions are type-checked and
recorded in the variable table. They are not runtime object dispatch.

```text
better_wall = wall with {
    u_value = 100 W/K
}
```

Copy-with creates a new object summary from an existing object plus explicit
field overrides. The source object is not mutated. Copied fields and overrides
remain visible in `object_summary`.

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

## Static Module Objects

Top-level object literals and copy-with values are immutable metadata values and
cross a static file-import boundary with class declarations:

```text
use "materials.eng"
wall_u = standard_wall.u_value
```

The imported object keeps its original source span, explicit fields, copied
fields, overrides, source-object provenance, class validation results, and
parallel type/hover metadata. Object names share one compilation-wide binding
namespace; a duplicate imported or root name is an error. Arbitrary top-level
`name = expression` executable bindings are still not imported.

The bounded editor recheck may change a final fast-binding suffix between a
direct registered-scalar field and a zero-argument method:

```text
wall_u = standard_wall.conductance()
scaled_u = wall_u * 0.5
```

This path first reconstructs and verifies the preserved object metadata, then
reanalyzes the changed member binding and later scalar dependencies. A member
inside a larger expression, an argument-bearing method, an unknown member, or a
non-scalar field uses full analysis.

## Diagnostics

| Diagnostic | Trigger |
|---|---|
| `E-CLASS-OBJECT-001` | Object literal references an unknown class. |
| `E-CLASS-OBJECT-002` | Object field is not attached to an object literal. |
| `E-CLASS-OBJECT-DUPLICATE-001` | Object name conflicts with an earlier imported or root binding. |
| `E-CLASS-FIELD-MISSING-001` | Object omits a required class field. |
| `E-CLASS-FIELD-UNKNOWN-001` | Object sets a field not declared by its class. |
| `E-CLASS-FIELD-TYPE-001` | Field value has an incompatible type or quantity. |
| `E-CLASS-FIELD-TYPE-002` | Class field uses an unknown type. |
| `E-CLASS-VALIDATION-001` | Class validation rule is not a supported comparison. |
| `E-CLASS-VALIDATION-002` | Object literal fails a class validation rule. |
| `E-CLASS-METHOD-SELF-001` | Method return expression cannot resolve `self.<field>`. |
| `E-CLASS-METHOD-RETURN-001` | Method return expression type does not match declaration. |
| `E-CLASS-METHOD-CALL-001` | Method call references an unknown object. |
| `E-CLASS-METHOD-CALL-002` | Method call references an unknown method. |
| `E-CLASS-METHOD-CALL-003` | Method call passes arguments to a zero-argument method. |
| `E-CLASS-COPY-001` | Copy-with references an unknown source object. |

Diagnostic fixtures live under `examples/diagnostics/error_messages/`:

- `class_missing_field.eng`
- `class_unknown_field.eng`
- `class_field_type_mismatch.eng`
- `class_validation_fail.eng`
- `class_method_return_mismatch.eng`
- `class_method_unknown.eng`
- `class_copy_unknown_source.eng`

## Artifact Surface

`eng check --review` writes class and object information to `review.json`:

```bat
target\debug\eng.exe check examples\official\19_class_object\main.eng --review
```

`eng run --save-artifacts` carries the same metadata into `report_spec.json`
and `report.html`:

```bat
target\debug\eng.exe run examples\official\19_class_object\main.eng --save-artifacts
```

The artifact sections are:

```text
class_summary
object_summary
```

`class_summary` includes validation rules and method declarations.
`object_summary` includes copy-with provenance plus per-object validation
results with left/right values, unit, and status.

The native IDE object summary inspector, artifact outline, and LSP snapshot path
expose the same sections, plus class/object hover and completion metadata. LSP
field completion marks required fields and class defaults, both for `object.`
member completion and for fields inside an object literal or copy-with block.

## Support Boundary

Current:

- parser and AST support for `class` declarations;
- typed fields with default values;
- object literals;
- static import of top-level object literals and copy-with values;
- nested object references;
- field access type metadata;
- simple class validation blocks with object-level pass/fail artifacts;
- zero-argument `method` declarations returning direct `self.<field>` values;
- method call type metadata;
- immutable copy-with object metadata;
- missing, unknown, and incompatible field diagnostics;
- review/report artifact sections;
- official example and CLI smoke coverage;
- IDE keyword/snippet completion and artifact-outline visibility;
- LSP class/object hover and field completion metadata with required/default
  marks.

Deferred:

- mutation;
- runtime object dispatch/lowering;
- inheritance;
- class-contained `port`/`connect` declarations.
