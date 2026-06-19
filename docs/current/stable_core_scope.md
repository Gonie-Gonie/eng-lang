# Stable Core Scope

EngLang `1.0.0` is a stable-core release, not a claim that every implementation
seed on `main` is complete and not a claim of a complete engineering simulation
solver. The stable contract is intentionally narrow: the documented
data-to-report workflow, artifact family, package smoke path, and native tester
workflow are expected to remain compatible across `1.x` releases.

## What Is Stable In 1.0.0

- Top-level file execution without `script main`.
- Root `args { ... }` for String/path/CsvFile/DirectoryPath and primitive
  Bool/Int/Count/Float/Duration values.
- Fast `=` bindings, explicit quantity declarations, and `:=` rejection.
- Built-in quantity/unit registry used by the official examples.
- `degC` plus the `°C` alias for absolute temperature display.
- Typed CSV promotion for the official schema/data boundary.
- DateTime-indexed table metadata and the documented HeatRate TimeSeries path.
- TimeSeries statistics and trapezoidal integration for the documented data
  path.
- Measured-vs-simulated workflow: CSV-derived measured/weather TimeSeries,
  explicit `TimeSeries[Time]` thermal input contract, one-state fixed-step
  thermal simulation output as `sim.T_zone`, RMSE metric, validation result,
  time-alignment metadata, and multi-series PlotSpec.
- Unit-aware `print`, structured `log <level>`, one-row summary CSV export,
  explicit write outputs, process results, and local test/assert/golden checks
  within their documented boundaries.
- Explicit side-effect artifacts: `output_manifest.json`, `run_log.json`,
  `process_results.json`, and `test_results.json`.
- `eng run --profile safe|normal|repro` basics: safe rejects explicit workflow
  write/export/file-operation/process effects, normal is the default, and repro
  records profile diagnostics in result/run-log/output-manifest artifacts.
- PlotSpec v1 line and multi-series line plots, SVG output, report HTML,
  review JSON, report spec, run log, process results, test results, and output
  manifest artifacts.
- Standalone packaged runner with `.engpkg`, `.lock`, Args help, dependency
  copying, package smoke, curated PDF docs, release zip, and SHA256 checksum.
- Tauri/WebView tester IDE smoke path, terminal/variables/plot/artifact
  inspection, and on-demand report/plot opening for the stable workflow.

## Stable Artifact Family

The stable-core artifact family is:

```text
.engbc
.engres
review.json
report.html
report_spec.json
plot_spec.json
plot_manifest.json
timeseries.svg
run_log.json
process_results.json
test_results.json
output_manifest.json
```

These artifact formats remain subject to the documented schema/version headers
and stable-core breaking-change policy.

## What Is Not Stable

The package may contain supported features or internal implementation seeds.
They may change in `1.x` releases as long as the stable-core contract above is
preserved:

- State-space matrix simulation. The supported typed-block discrete/continuous
  fixed-step workflows are supported scoped additions, not stable-core claims.
- Class runtime object dispatch, simulation lowering, method arguments,
  mutation, and inheritance.
- Production component graph numeric solving and physical multi-domain solving.
  The constrained Thermal component boundary assembly is a supported scoped
  addition, the narrow fixed-point ResidualGraph source path is a supported
  scoped addition, the simple-linear dynamic component source path is a
  supported scoped addition, and the narrow source Newton/implicit-Euler DAE
  component residual smokes are supported scoped additions; none of them are
  stable-core claims.
- Broad nonlinear/DAE simulation, broad adaptive, or general multi-state
  equation solving. The one-state thermal `adaptive_heun` path, the supported
  two-state source-equation fixed-step path, the narrow source
  Newton/implicit-Euler DAE component residual smokes, and internal
  state-space seeds are outside the stable-core claim.
- LSP/VS Code as a stable persistent editor-service contract.
- Native JIT/AOT execution or speedup claims.
- Domain package registry or open component ecosystem.
- Broad TimeSeries/table expression execution beyond the documented path.
- Uncertainty and data-driven modeling engines as stable features.
- Full filesystem/network support and full process sandboxing.
- Workspace-wide test discovery, filtering, and fixtures.

## Breaking-Change Boundary

The breaking-change policy applies to stable syntax, stable CLI behavior, stable
runtime artifact headers/sections, stable package layout, and the documented
stable workflows above. Internal implementation seeds can evolve faster, but
release notes must not present them as stable user-facing behavior.

## Stable-Core Maintenance Gate

Before a stable-core maintenance slice is accepted:

```text
.\dev.bat ci
.\dev.bat docs-check
.\dev.bat artifacts-check
.\dev.bat release-check
```

The `eng test examples` gate directly exercises the official run/build paths,
Korean and space-containing paths, standalone package execution under a
sanitized Rust/Python-free child-process PATH,
CSV source-hash provenance, TimeSeries axis and HeatRate-to-Energy integration
metadata, measured-vs-simulated SolverResult method/timestep/final-state
metadata, RMSE/validation units, measured-vs-simulated repro-profile saved
artifacts, side-effect artifacts, safe-profile rejection of explicit
export/write/file/process effects, normal-profile process/test/output-manifest
fields, and repro-profile diagnostics in saved artifacts.

The release note must distinguish Stable, Supported, Internal, and Planned
behavior. Package smoke must pass from a clean extracted folder without Rust,
Python, Node, or Visual Studio Build Tools on the target side.
