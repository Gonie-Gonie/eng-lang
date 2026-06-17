# Historical Stable-Core Backfill Audit

This audit is implementation history. It records work that was originally
tracked under an older high-numbered planning line, but the active public
version policy is now:

```text
public releases: v0.1-preview, v0.2-preview, ...
development scope: named tracks
stable core: reserved until behavior and support are genuinely stable
```

Use this file to understand why certain historical implementation seeds exist. Do not use it
as the active release roadmap; use `docs/current/version_plan.md`,
`docs/current/tracks.md`, and `docs/current/status.md` instead.

## Status Terms

```text
Implemented
  Behavior exists, has a regression/smoke gate, and is documented.

Internal
  Behavior is useful for implementation testing but intentionally narrower than
  the eventual full engine and not presented as a release feature.

Deferred
  The shape is known, but the implementation remains outside the current
  public release support boundary.
```

## Completed Backfills

| Area | Implemented detail | Gate |
| --- | --- | --- |
| Documentation examples | Supported docs code blocks are checked instead of treated as prose-only examples. | `.\dev.bat docs-check` |
| Official examples | `examples/official` is the release-critical example suite for CSV/report, simple system, integrated HVAC, uncertainty, data-driven modeling, and domain/component metadata. | `.\dev.bat artifacts-check`, `.\dev.bat package-smoke` |
| Data boundary | Official CSV promotion validates headers, required columns, schema constraints, missing policies, row-level table pages, and source hashes. | artifact golden baselines |
| Unit conversion | CSV source-unit to canonical-unit conversion metadata is carried in review/report/result artifacts, with per-cell conversion failures. | artifact golden baselines |
| TimeSeries values | The official coil heat-rate path materializes runtime TimeSeries pages from CSV data. | runtime smoke and artifacts-check |
| Statistics | `mean`, `time_weighted_mean`, `min`, `max`, `median`, `std`, `pNN`, `duration_above`, and trapezoidal integration run for the supported official TimeSeries path. | runtime smoke and artifacts-check |
| Plot/report | Plot blocks execute supported `unit`, `type`, and `title` options for official line, bar, and histogram paths. PlotSpec, SVG, report spec, HTML report, and plot manifest are generated without Python. | artifacts-check and package-smoke |
| System/equation | System metadata includes variables, equations, residuals, solver boundary, solver plan, dependency data, derivative states, a fixed-step path for the official one-state thermal system, and solver result snapshots for variables, diagnostics, and trajectory points across review/report/result artifacts. | system artifact golden baselines |
| Standalone package | `eng build --standalone --profile repro` creates a runnable package with bytecode, lock metadata, argument help, dependencies, and a `run.bat` wrapper. | package-smoke |
| Native IDE | The tester IDE opens files, runs checks, runs examples, shows diagnostics/completions/symbols/results, supports settings, and is bundled as `eng-ide.exe`. | `.\dev.bat ide-check`, package-smoke |
| User docs | Portable release docs are curated into a PDF guide; developer markdown is not bundled into the user-test package. | package-smoke |

## Remaining Deferrals

These items must stay documented as Internal or Planned until they are
implemented and gated:

```text
- adaptive, nonlinear, and multi-equation numeric system solvers
- broad TimeSeries expression execution beyond supported official paths
- full stochastic uncertainty propagation engine
- production-grade data-driven modeling engine and model persistence
- public-support LSP/editor service contract
- optimized native JIT/AOT execution as a supported runtime path
- domain package ecosystem with compatibility policy
- stable binary report package and interactive report viewer
```

## Documentation Rule

When describing these features in public docs:

```text
Say:
  current supported scope
  internal implementation seed
  uncertainty track
  data-driven modeling track
  IDE/LSP track
  runtime optimization track
  domain/component track

Avoid:
  using old high-numbered planning labels as public release names
  claiming stable-core support before the stable gate is met
  implying implementation seeds are complete engines
```

## Next Audit Point

Run a fresh gap audit before any future stable promotion. The audit should start
from the current track files and official examples, not from the historical
milestone labels in the long-form master plan.
