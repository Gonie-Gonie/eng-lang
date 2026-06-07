# v1.1 Uncertainty Core Gate

This page tracks the concrete gate for treating the v1.1 uncertainty surface as
more than an implementation seed. It does not by itself make v1.1
release-supported; the feature maturity matrix remains authoritative until the
release target is changed.

## Current Scope

The current v1.1 scope is intentionally narrow and deterministic:

- Measured values: `measured(value, std=...)`.
- Intervals: `interval(lower, upper)` or named `lower=`/`upper=` bounds.
- Distributions: `normal(mean=..., std=..., samples=n)`,
  `uniform(lower, upper, samples=n)`, and `distribution(kind=normal|uniform,
  ...)`.
- Ensembles: `ensemble(source, samples=n)` from a prior uncertainty binding.
- Propagation: `propagate(source, method=linear, scale=..., offset=...)` from a
  prior uncertainty binding.
- Samples: deterministic, bounded to the current compiler/runtime contract of
  `1..=256`.
- Plots: `plot distribution(source)` produces histogram PlotSpec/SVG artifacts
  with reviewable bin metadata.

## Completed On Main

- [x] Compiler records `UncertaintyInfo` for measured values, intervals,
  normal/uniform/distribution calls, ensembles, and propagation bindings.
- [x] Compiler semantic types are recorded as `Measured[T]`, `Interval[T]`,
  `Distribution[T]`, and `Ensemble[T]`.
- [x] Compiler source diagnostics validate the staged uncertainty chain:
  uncertainty binding -> ensemble/propagate source.
- [x] Compiler argument diagnostics validate required numeric values, standard
  deviations, bounds, sample-count range, supported distribution kinds, linear
  propagation method, and numeric scale/offset values.
- [x] Runtime materializes deterministic samples, summaries, percentiles, and
  histogram bins.
- [x] Runtime applies deterministic linear propagation with scale/offset
  transform metadata.
- [x] Review/report/result artifacts expose uncertainty metadata through
  `review.json`, `report_spec.json`, `result.engres`, `report.html`, and
  PlotSpec/SVG artifacts.
- [x] Native IDE Runtime Summary shows uncertainty kind, distribution/method,
  source, sample count, p05/p50/p95, and transform metadata from
  `result.engres`.
- [x] Official example:
  `examples/official/04_uncertainty_core/main.eng`.
- [x] CLI `eng test examples` covers uncertainty source diagnostics, argument
  diagnostics, result metadata, report metadata, and histogram PlotSpec output.

## Remaining Before Support Claim

- [ ] Decide whether v1.1 should be promoted from Experimental to Preview or
  Supported for a release target.
- [ ] Write release notes for the exact public scope and limitations.
- [ ] Keep full Monte Carlo engines, covariance matrices, Jacobian propagation,
  correlated variables, posterior inference, and optimization workflows
  explicitly deferred.
- [ ] Keep runtime unit conversion inside uncertainty samples narrow; the
  compiler records display units, but v1.1 does not claim a full uncertainty
  unit algebra.

## Diagnostic Contract

```text
E-UNC-SOURCE-001  missing or unknown uncertainty source reference
E-UNC-SOURCE-002  referenced binding exists but is not an uncertainty source
E-UNC-ARGS-001    missing or malformed required uncertainty argument
E-UNC-ARGS-002    invalid numeric, range, count, or transform argument
E-UNC-ARGS-003    unsupported uncertainty option for the current v1.1 scope
```

## Verification

```bat
.\dev.bat release-check
target\debug\eng.exe run examples\official\04_uncertainty_core\main.eng --entry main
target\debug\eng.exe check examples\05_error_messages\missing_uncertainty_source.eng --review
target\debug\eng.exe check examples\05_error_messages\invalid_uncertainty_arguments.eng --review
```
