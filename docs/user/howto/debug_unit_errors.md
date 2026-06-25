# Debug Unit Errors

When a unit diagnostic appears, fix the earliest mismatch instead of removing
units.

Common checks:

- Did the literal need an explicit unit?
- Is a temperature delta written as K rather than degC?
- Is the requested display unit compatible with the quantity?
- Did a CSV schema column declare the right quantity kind and source unit?

For reference material, read docs/reference/language/dimensionless.md and
docs/reference/language/syntax.md.
