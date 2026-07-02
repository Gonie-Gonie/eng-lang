# Report Language

Report blocks collect values, tables, plots, and generated artifacts into the
reviewable report output.

## Supported Commands

```englang
report {
    show summary
    plot heat over Time
    with {
        unit y = kW
        title = "Heat rate"
    }
}
```

- `show <value>` adds a value, table summary, or artifact reference to the
  report.
- `plot <series> over Time` creates a plot spec, SVG, plot manifest, and report
  entry.
- Plot options include `unit y`, `unit x`, `title`, and `confidence_band` where
  the source value is available.

## Generated Artifacts

Saved runs write report data under `build/result`, including `report_spec.json`,
`report.html`, plot specs, plot SVG files, and plot manifests.

## Related References

- [Plotting](plotting.md)
- [Report and review artifacts](../artifacts/report_review.md)
- [Side-effect policy](side_effect_policy.md)
