# Units

EngLang treats units as part of the type contract. The compiler checks
arithmetic, schema promotion, display units, and plot/report options against the
quantity kind carried by each value.

## Supported Use

- Write source units in square brackets for schema columns and typed
  declarations, for example `HeatRate [kW]`.
- Write numeric literals with explicit units when dimensional meaning matters,
  for example `12 kW`, `30 s`, or `2 MB`.
- Use compatible display units in plot/report options such as `unit y = kW`.
- Use `1` for dimensionless schema units and dimensionless plot axes.

## Diagnostics To Expect

- Addition and subtraction require compatible quantity kinds.
- Ambiguous power, temperature, or dimensionless literals should be annotated
  with an explicit quantity type or unit.
- Plot/report display units must match the expression quantity.

## Related References

- [Dimensionless and unit policy](dimensionless.md)
- [Syntax and grammar](syntax.md)
- [Plotting](plotting.md)
