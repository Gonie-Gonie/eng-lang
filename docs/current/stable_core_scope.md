# Public Package Scope

This file preserves the old stable-core link target, but the current published
package line is `v0.1.0`. Public claims should be based on concrete package
scope and maturity status, not on old version ladders.

`v0.1.0` is an initial portable release for the documented CLI/data/report
workflow. It is not a complete engineering simulation solver.

## Public Package Scope

- Top-level file execution without `script main`.
- Root `args { ... }` for String/path/CsvFile/DirectoryPath, primitive
  Bool/Int/Count/Float values, and unit-aware registry quantities.
- Fast `=` bindings, explicit quantity declarations, and `:=` rejection.
- Built-in quantity/unit registry used by the official examples.
- `degC` plus the `°C` alias for absolute temperature display.
- Typed CSV promotion for the official schema/data boundary.
- DateTime-indexed table metadata and the documented HeatRate TimeSeries path.
- TimeSeries statistics and trapezoidal integration for the documented data
  path.
- Unit-aware `print`, structured `log <level>`, one-row summary CSV export,
  explicit write outputs, process results, file-operation metadata, and local
  test/assert/golden checks within their documented boundaries.
- Explicit side-effect artifacts: `output_manifest.json`, `run_plan.json`, `run_log.json`,
  `process_results.json`, and `test_results.json`.
- `eng run --profile safe|normal|repro` basics.
- PlotSpec v1 line and multi-series line plots, SVG output, report HTML,
  review JSON, report spec, run plan, run log, process results, test results,
  and output manifest artifacts.
- Standalone packaged runner with `.engpkg`, `.lock`, Args help, dependency
  copying, package smoke, curated PDF docs, release zip, and SHA256 checksum.
- Portable native IDE smoke path and packaged LSP smoke/snapshot tooling.

## Artifact Family

The package artifact family is:

```text
.engbc
.engres
review.json
report.html
report_spec.json
plot_spec.json
plot_manifest.json
timeseries.svg
run_plan.json
run_log.json
process_results.json
test_results.json
output_manifest.json
```

Artifact formats remain subject to the documented schema/version headers and
breaking-change policy.

## Outside The Public Package Claim

The repository may contain supported features or internal implementation tracks
on `main` that are newer than the published `v0.1.0` assets:

- General nonlinear/DAE simulation, broad behavior graph solving, broad
  adaptive solving, and general multi-state equation solving.
- Production component graph numeric solving and physical multi-domain solving.
- production pressure-drop packages and a domain package registry.
- Native JIT/AOT execution or speedup claims.
- LSP/VS Code as a stable persistent editor-service contract.
- Uncertainty and data-driven modeling engines as stable features.
- Full filesystem/network support and full process sandboxing.
- Workspace-wide test discovery and filtering.

Scoped solver examples on `main` must stay described by their actual evidence:
source syntax, compiler checks, runtime numerical result, residual/RHS/failure
artifacts, report/review/IDE visibility, official examples, and success/failure
tests.

## Acceptance Gate

Before a package or solver-centered slice is accepted, run the relevant local
gate and keep release notes/status docs aligned:

```text
.\dev.bat ci
.\dev.bat jit-check
.\dev.bat docs-check
.\dev.bat artifacts-check
.\dev.bat release-check
```

Use the lighter subset that matches the change while developing, then run the
full release gate before publishing package assets.
