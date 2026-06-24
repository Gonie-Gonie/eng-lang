# Uncertainty And Distribution Numeric Track

Status: internal implementation track. This page is a current-scope contract
for contributors, not a public package feature claim.

EngLang numeric and quantity values should be uncertainty-capable when the
workflow needs that meaning. A deterministic scalar is still the common fast
path, but the semantic model should be able to describe it as the certain case
of a broader numeric value model.

## Philosophy

Engineering values are often measurements, estimates, calibration outputs, or
simulation summaries. Treating every value as only a naked scalar hides the
review question that matters: how certain is the value, and how did that
uncertainty affect the result?

The intended model is:

```text
Certain scalar                -> fast runtime scalar path
Measured value with error     -> value plus measurement metadata
Interval value                -> lower and upper bound
Distribution value            -> named distribution and parameters
Ensemble value                -> explicit deterministic or sampled members
```

This does not mean every scalar must allocate an uncertainty payload. It means
the type, runtime, artifact, and IDE layers should be able to carry uncertainty
when it exists without changing the meaning of deterministic workflows.

## Current Implementation Status

Current internal evidence exists in:

```text
docs/guide/uncertainty.md
examples/internal/04_uncertainty_core
examples/diagnostics/error_messages/invalid_uncertainty_arguments.eng
examples/diagnostics/error_messages/missing_uncertainty_source.eng
crates/eng_compiler/src/uncertainty.rs
```

The current seed supports deterministic uncertainty constructors, selected
diagnostics, scalar runtime numeric payloads, histogram artifacts, and
report/review projection for the internal fixture. `result.engres` now records
`typed_payload.numeric_values` so certain scalars stay on the fast path while
measured, interval, distribution, and ensemble scalars carry an uncertainty
link. It is not yet a stable propagation contract for arbitrary arithmetic,
TimeSeries uncertainty, or seeded Monte Carlo workflows.

## Representation Target

The semantic representation should distinguish uncertainty kind from physical
quantity and unit metadata:

```text
UncertaintyRep
  Certain
  Measured(std, error, confidence)
  Interval(lower, upper)
  Distribution(kind, params, samples)
  Ensemble(samples)
```

Quantity kind, display unit, canonical unit, axis metadata, schema metadata,
and uncertainty representation are separate concerns. A `HeatRate [kW]` value
may be certain, measured, interval-valued, distribution-valued, or ensemble
valued.

The implemented scalar runtime slice records:

```text
typed_payload.numeric_values[].representation
typed_payload.numeric_values[].uncertainty
object_store.objects[].numeric
```

Deterministic scalars use `representation = "Certain"` and do not allocate a
sample payload. TimeSeries and table-level uncertainty remain future work.

## Syntax Policy

Keep the initial surface constructor-based:

```eng
T_supply = measured(12 degC, std=0.2 K)
L_sensor = measured(10 m, error=1 %)
Q_band = interval(4.8 kW, 5.4 kW)
Q_dist = normal(mean=5 kW, std=0.8 kW, samples=31)
Q_uniform = uniform(4 kW, 6 kW, samples=21)
Q_ensemble = ensemble(Q_dist, samples=31)
Q_adjusted = propagate(Q_dist, method=linear, scale=1.08, offset=0.4 kW)
```

Unicode or shorthand forms such as `+/-` should remain deferred until the
constructor contract, diagnostics, report fields, and IDE display are aligned.

## Propagation Policy

Propagation must be explicit and reviewable. The target option model is:

```eng
Q = m_dot * cp_water * dT
with {
    uncertainty = linear
}
```

Allowed policy names should start narrow:

```text
linear
interval
monte_carlo
ensemble
```

The implemented preview already materializes deterministic scalar arithmetic
for uncertain sources, for example `Q_total = Q_meas + 2 kW`, by evaluating
same-index samples and recording a linear or interval arithmetic propagation
status. This is deliberately narrower than a general symbolic Jacobian,
Monte Carlo engine, or full deterministic-binding value evaluator.

Compiler review now accepts and validates `with { uncertainty = ... }` policy
metadata. Allowed policy names are `linear`, `interval`, `monte_carlo`, and
`ensemble`; `samples` must be a positive integer; `seed` must be a
non-negative integer when present; and `monte_carlo` without a seed records a
reproducibility warning. `review.json.uncertainty_policies[]` is the normalized
review surface for this metadata.

When a policy assumes independence, linearizes a nonlinear expression, or falls
back to a lower-fidelity rule, that assumption belongs in `review.json`,
`report_spec.json`, `report.html`, and the IDE warning panel.

## Validation Semantics

Direct comparison of an uncertain value should not silently mean comparison of
its nominal value. A validation must name the review statistic:

```eng
validate p95(Q) < 10 kW
validate probability(Q < 10 kW) > 0.95
validate mean(Q) between 4 kW and 6 kW
```

Until these forms are fully implemented, direct uncertain comparisons should
stay diagnostic-only or internal.

## Report And IDE Requirements

A supported uncertainty workflow needs review fields for:

```text
binding
quantity kind
display unit
representation
mean or nominal value
standard deviation
interval lower and upper
p05, p50, p95
propagation method
sample count
seed
assumptions
warnings
```

IDE surfaces should expose the same meaning as tables, hovers, validation rows,
histogram previews, and confidence-band previews. Raw JSON is not enough for a
reviewability-centered feature.

## Non-Goals

The uncertainty track should not:

```text
claim stable Monte Carlo semantics before seeded reproducibility is enforced
hide propagation assumptions inside runtime code
make every deterministic scalar pay an uncertainty allocation cost
promote uncertainty to the public package before artifacts and IDE agree
invent domain-specific measurement models in the core language
```

## Completion Checklist

Move an uncertainty slice from internal to supported only when these are aligned:

```text
language rule
parser and AST support
semantic type, unit, axis, and schema checks
runtime representation and propagation behavior
diagnostic code and source span
review/report artifact fields
IDE hover/table/plot metadata
official or workflow example
compile, runtime, diagnostic, and artifact tests
README, status, and maturity matrix entries
```
