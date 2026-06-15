# Official 19 Class Object Preview

Metadata-first class/domain-object preview for reviewable engineering library
objects.

This example shows:

- `class` declarations with typed fields and default field values.
- Class `validate` blocks for simple object invariants.
- Object literals such as `wall = Construction { ... }`.
- Required field checking for fields without defaults.
- Nested object fields, where `Building.zone` references a `Zone` object and
  `Building.envelope` references a `Construction` object.
- A `WeatherData` metadata object that shows how library objects can reference
  data-source contracts without runtime lowering yet.
- Field access such as `wall.u_value`, which is typed as `Conductance [W/K]`.
- `class_summary` and `object_summary` metadata in review/report artifacts.

Current support boundary:

- class/object parsing, metadata, object literal diagnostics, simple class
  validation rules, field access typing, report/review sections, and IDE
  artifact outline visibility are supported for this preview;
- methods, copy-with, inheritance, mutation, and runtime object dispatch are
  deferred;
- classes are ordinary typed engineering objects, not replacements for
  `system`, `component`, `schema`, or `domain`.

Useful commands:

```bat
target\debug\eng.exe check examples\official\19_class_object_preview\main.eng --review
target\debug\eng.exe run examples\official\19_class_object_preview\main.eng --save-artifacts
```
