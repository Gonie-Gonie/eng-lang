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
.\dev.bat release-check
```

`release-check` runs `ci`, `package-smoke`, zip existence checks, checksum
verification, and `dist\release-manifest.txt` generation.

`package` writes both a portable zip and checksum:

```text
dist\englang-preview-v<version>-windows-x64.zip
dist\englang-preview-v<version>-windows-x64.zip.sha256
```

`package-smoke` extracts that zip into a path containing spaces and Korean
characters, then runs the packaged `eng.exe` without using Rust or Python from
the target folder.

Optional manual smoke from `dist\englang-preview`:

```bat
pushd dist\englang-preview
eng.exe doctor
eng.exe entries examples\04_plotting\main.eng
eng.exe run examples\04_plotting\main.eng --entry main
eng.exe view build\result\result.engres
eng.exe run examples\06_simple_system\main.eng --entry main
eng.exe view build\result\result.engres
type build\result\plots\plot_spec.json
type build\result\plots\plot_manifest.json
type build\result\report_spec.json
eng.exe check examples\05_error_messages\missing_csv_column.eng --review
eng.exe check examples\05_error_messages\heat_rate_sum.eng --review
eng.exe check examples\05_error_messages\eq_boolean.eng --review
eng.exe check examples\05_error_messages\equation_unit_mismatch.eng --review
eng.exe run examples\05_error_messages\missing_entry.eng
eng.exe build examples\02_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat
popd
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

## v0.8 Gate

```text
[x] system block parses
[x] parameter/state/input variables appear in review.json
[x] parameter/state/input variables appear in report_spec.json
[x] eq relation checks unit consistency
[x] der() contributes derivative dimension metadata
[x] == in equation block produces E-EQ-BOOL-001
[x] mismatched equation dimensions produce E-EQ-UNIT-001
[x] residual metadata appears in review.json
[x] residual metadata appears in report_spec.json and result.engres
[x] report.html includes System Equations
[x] official simple system example passes
```

## v0.9 Gate

```text
[x] package creates dist\englang-preview
[x] package creates dist\englang-preview-v<version>-windows-x64.zip
[x] package creates dist\englang-preview-v<version>-windows-x64.zip.sha256
[x] package includes eng.exe, examples, stdlib, docs, and README.txt
[x] package-smoke extracts the zip under a path with spaces and Korean characters
[x] packaged eng.exe doctor passes in the extracted folder
[x] packaged CSV+plot example runs and creates result/report/PlotSpec artifacts
[x] packaged simple system example runs and creates result/report artifacts
[x] eng test examples includes a Korean and space-containing source/build path smoke
[x] no Python/Rust install is required for packaged preview execution
```

## v1.0 Demo Direction

The v1.0 demo must show:

```text
[x] typed CSV boundary
[x] unit/quantity-aware calculations
[x] TimeSeries statistics
[x] PlotSpec-driven SVG/report
[x] reviewable result/report/provenance
[x] packaged or portable execution
```

## v1.0 Gate

```text
[x] workspace version is 1.0.0
[x] official examples pass through eng test examples
[x] official CSV+plot example produces report and PlotSpec artifacts
[x] official simple system example produces system report artifacts
[x] standalone build creates dist\<model>-standalone
[x] standalone bundle includes eng.exe, run.bat, source, bytecode, engpkg, lock, and review
[x] standalone .engpkg uses format = engpkg-stable-1
[x] standalone lock records bytecode/result/report/plot format versions
[x] standalone run.bat creates report and PlotSpec artifacts inside the bundle
[x] package-smoke verifies portable zip execution and standalone packaged runner execution
[x] no Python/Rust install is required for portable or standalone packaged execution
```

Release notes live in `docs/release/v<version>.md`.

The full release procedure lives in [release-workflow.md](release-workflow.md).

Post-v1.0 implementation gaps and seed-only areas are tracked in
[v1.0 gap audit](../development/05_v1_0_gap_audit.md). New release gates should
pull from that register before claiming additional stable behavior.
