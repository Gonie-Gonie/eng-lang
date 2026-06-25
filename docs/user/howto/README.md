# How-To Guides

Use these task guides after you have run the first tutorials.

## Read CSV And Plot A TimeSeries

Start from examples/official/01_csv_plot/main.eng.

1. Define a schema with typed columns and a DateTime index.
2. Use promote csv args.input as YourSchema.
3. Derive a TimeSeries with unit-aware expressions.
4. Add a report block with plot ... over Time.
5. Run with a command such as eng.exe run source.eng --out build/runs/name.

Inspect review.json for schema promotion, TimeSeries metadata, and plot
artifact paths.

## Create A Report

Use a report block when a result needs a human-readable artifact:

```eng partial
report {
    summarize Q_coil by [mean, max, p95]
    show E_coil
    plot Q_coil over Time
}
```

Treat report.html as presentation and review.json as evidence. Review both
before accepting a result.

## Run An External Command

Use run command only when the boundary is intentional:

```eng partial
result = run command "cmd"
with {
    args = ["/C", "echo", "eng-process-ok"]
}
```

The review artifact should record the command boundary, arguments, status, and
related outputs. For policy details, read docs/reference/side_effect_policy.md.

## Save Artifacts

Use explicit write statements and overwrite policy:

```eng partial
write json "outputs/energy.json", E_coil
with {
    overwrite = true
}
```

Generated files should appear in the output manifest or review artifact so a
reviewer can distinguish intended outputs from incidental files.

## Review LLM-Generated Code

Run LLM-generated EngLang code through the same path as human-written code:

```bat
eng.exe check path/to/candidate.eng
eng.exe run path/to/candidate.eng --out build/runs/candidate
```

Reject code that removes units, hides input paths, skips schema promotion, or
produces a report without reviewable evidence.

## Use The Native IDE

Open a source file with:

```bat
eng.exe ide examples/official/01_csv_plot/main.eng
```

Use the IDE to inspect diagnostics, runtime objects, schemas, unit conversions,
artifacts, TimeSeries metadata, and report/review surfaces together.
