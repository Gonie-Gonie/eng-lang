# Summary Object Decision

Status: deferred during the v0.2 planning scope.

EngLang currently supports summary output through explicit artifact statements,
not through a first-class `Summary` value.

Current supported surface:

```eng
E_total = 42.75 kWh
Q_peak = 7.891 kW

print "E total = {E_total: .2 kWh}"

export summary to csv "summary.csv" {
    E_total as kWh with ".2"
    Q_peak as kW with ".2"
}
```

## Decision

Do not add a first-class Summary object in the v0.2 planning scope.

Reasons:

- `print` is intentionally debug/CLI output.
- `export summary to csv` is an explicit reproducible artifact request.
- Summary objects would need value semantics, display policy, report rendering,
  CSV layout, and IDE inspection rules before they are useful.
- Adding a thin `summary = ...` shell would blur the language boundary without
  solving those rules.
- The current explicit export block avoids accidental file generation and keeps
  artifact creation opt-in.

## Current Meaning Of `summary`

In v0.2 planning, `summary` is only the source selector in:

```eng
export summary to csv "summary.csv" {
    ...
}
```

It is not a variable, collection type, report object, or table-like value.

## Future Shape To Revisit

A future design may introduce a real object only if it can support:

- named fields with quantity and display-unit contracts;
- deterministic print/show/report/export behavior;
- IDE variable-panel inspection;
- explicit save/export commands;
- no implicit artifact writes;
- stable JSON/report schema representation.
