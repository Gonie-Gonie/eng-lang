# Release Acceptance Checklist

Every milestone slice should answer these questions before commit and release.

## Master Checklist

```text
1. Which target version does this change serve?
2. Which area changed: language, compiler, runtime, tooling, product, docs, examples, release?
3. Does the change avoid adding Python or external interpreter dependencies to the core path?
4. Does it respect the entry point and typed Args policy?
5. Does it avoid top-level side effects in file run/build?
6. Does it preserve v8/v9 syntax policy, including fast `=` and no `:=`?
7. Does it update examples for public behavior?
8. Does it update README, CLI docs, artifact docs, and release notes when behavior changes?
9. Does it update result/report/provenance contracts when artifacts change?
10. Does it add focused tests for new compiler/runtime behavior?
11. Does it keep generated artifacts reviewable by humans and tooling?
12. Does it pass `.\dev.bat ci`?
```

## Preview Release Commands

Run from the repository root:

```bat
.\dev.bat clean
.\dev.bat setup
.\dev.bat ci
.\dev.bat package
```

Smoke the portable package from `dist\englang-preview`:

```bat
eng.exe doctor
eng.exe entries examples\04_plotting\main.eng
eng.exe run examples\04_plotting\main.eng --entry main
eng.exe view build\result\result.engres
type build\result\plots\plot_spec.json
type build\result\plots\plot_manifest.json
type build\result\report_spec.json
eng.exe check examples\05_error_messages\missing_csv_column.eng --review
eng.exe check examples\05_error_messages\heat_rate_sum.eng --review
eng.exe run examples\05_error_messages\missing_entry.eng
```

The missing-entry command should fail with `E-ENTRY-NOT-FOUND-001`.

## v0.4 Gate

```text
[x] .engbc generated
[x] .engbc has bytecode version header
[x] bytecode encode/decode test
[x] VM scalar execution test
[x] VM array value seed test
[x] result.engres generated
[x] result.engres has result format version
[x] file run requires an entry point
[x] `eng entries` lists script entries
[x] no Python dependency in core run path
```

## v0.5 Gate

```text
[x] TimeSeries[Time] of HeatRate is inferred for Q_coil
[x] axis metadata appears in review.json
[x] summary statistics metadata appears in review.json
[x] lazy summary cache key appears in result.engres
[x] integrate(HeatRate over Time) -> Energy metadata appears in result.engres
[x] HeatRate sum lint produces W-STATS-SUM-001
[x] TimeSeries object appears in bytecode and VM object store
[x] report.html includes axis/statistics/integration sections
```

## v0.6 Gate

```text
[x] official example creates PlotSpec v1
[x] official example creates plot manifest
[x] SVG export exists
[x] SVG plot has unit-aware axis labels
[x] result.engres records plot_spec_hash
[x] eng view lists plot manifest
[x] PlotSpec JSON/SVG unit tests pass
```

## v0.7 Gate

```text
[x] review.json generated with review_schema_version
[x] review.json includes variable_table
[x] review.json includes unit_conversion_table
[x] review.json includes schema_summary
[x] review.json includes warning_list
[x] report_spec.json generated with eng-report-spec-v1
[x] report_spec.json includes inferred_declaration_table
[x] report_spec.json includes plot_manifest path/hash
[x] result.engres records report_spec_hash
[x] eng view lists report_spec.json
[x] official plotting example produces report and PlotSpec artifacts
```

## v1.0 Demo Direction

The v1.0 demo must show:

```text
1. typed CSV boundary
2. unit/quantity-aware calculations
3. TimeSeries statistics
4. PlotSpec-driven SVG/report
5. reviewable result/report/provenance
6. packaged or portable execution
```

Release notes live in `docs/release/v<version>.md`.
