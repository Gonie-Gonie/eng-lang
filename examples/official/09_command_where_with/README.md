# Official 09 Command Where With

Purpose:

```text
Demonstrate parenthesis-light command-style workflow syntax together with
owner-local where bindings, with option blocks, print output, and CSV export.
```

Language features shown:

```text
- typed CSV schema with constraints and missing-value policy
- command-style integrate, mean, max, summarize, and plot
- where { ... } locals attached to the preceding integration owner
- with { method = ... } and with { unit y = ..., title = ... } option blocks
- unit-aware print interpolation
- export summary to csv with display units and formatting
```

Run:

```bat
target\debug\eng.exe check examples\official\09_command_where_with\main.eng --review
target\debug\eng.exe run examples\official\09_command_where_with\main.eng --save-artifacts
```

Expected artifacts:

```text
build/result/result.engres
build/result/review.json
build/result/report_spec.json
build/result/summary.csv
build/result/plots/plot_spec.json
build/result/report.html
```

Limitations:

```text
- command style is limited to supported built-in workflow verbs
- where locals are owner-local and do not escape to outer scope
- with options are validated per owner command, not a general configuration DSL
```

Related docs:

```text
docs/guide/language_grammar.md
docs/guide/timeseries_statistics.md
docs/guide/plotting.md
docs/reference/side_effect_policy.md
```
