# Kernel Plan Reference

`eng.exe jit-plan <file.eng>` emits experimental hot-kernel metadata for the
runtime optimization track. The output is a planning artifact only. It does not mean
native code has been generated, selected, cached, or executed.

The current format marker is:

```json
{
  "format": "eng-kernel-plan-v1"
}
```

## Stability Policy

`eng-kernel-plan-v1` is experimental and not public release-supported.
Within this format:

- Existing top-level keys keep their current type.
- Existing candidate keys keep their current type.
- New optional keys may be added.
- Consumers must ignore unknown keys.
- Removing a required key, changing a required key type, or changing the meaning
  of `backend`, `kind`, or `lowering_status` requires a new format marker.

## Top-Level Shape

```json
{
  "format": "eng-kernel-plan-v1",
  "backend": "interpreter-fallback",
  "backend_selection": {
    "requested": "auto",
    "selected": "interpreter-fallback",
    "status": "selected",
    "reason": "auto currently resolves to the interpreter fallback backend"
  },
  "candidate_count": 3,
  "candidates": []
}
```

`backend = "interpreter-fallback"` means the normal EngLang runtime still owns
execution. Current JIT planning makes no speedup claim.

## Backend Selection

Supported backend requests:

```text
auto
interpreter-fallback
native-preview
```

`auto` and `interpreter-fallback` select `interpreter-fallback`.

`native-preview` records that a native backend was requested, but the current
selection still falls back to `interpreter-fallback` with
`status = "not_available"`. No native backend is compiled, cached, selected for
execution, or benchmarked.

## Candidate Shape

```json
{
  "name": "E_coil",
  "kind": "timeseries_integrate",
  "line": 29,
  "source": "Q_coil",
  "reason": "HeatRate over Time lowers to a trapezoid-style numeric kernel",
  "lowering_status": "lowerable_to_numeric_kernel_plan",
  "operations": [
    "load_timeseries:Q_coil",
    "integrate_over:Time",
    "store:E_coil"
  ],
  "estimate": {
    "estimated_rows": 4,
    "input_count": 1,
    "output_count": 1,
    "operation_count": 2,
    "scan_count": 1,
    "complexity": "O(n) TimeSeries integration",
    "notes": [
      "adjacent samples form trapezoid intervals",
      "stores one integrated quantity"
    ]
  }
}
```

`line` is one-based and points at the source construct that produced the
candidate.

## Estimate Shape

`estimate` is a planning estimate, not a measured benchmark result.

```json
{
  "estimated_rows": 4,
  "input_count": 1,
  "output_count": 1,
  "operation_count": 2,
  "scan_count": 1,
  "complexity": "O(n) TimeSeries integration",
  "notes": []
}
```

`estimated_rows` is inferred from CSV promotion metadata when the candidate can
be traced to a TimeSeries source. It is `null` when row count is not known, such
as interface-only system residual planning.

`operation_count` is an operation-class count used for planning and inspection.
It is not a floating-point operation count and must not be used for performance
claims.

## Candidate Kinds

```text
timeseries_arithmetic
timeseries_integrate
statistics_fusion
system_residual
```

`system_residual` is currently interface-only. It reserves the RHS/Jacobian
lowering shape for later work and is not a native solver backend.

## Lowering Status

```text
lowerable_to_numeric_kernel_plan
interface_only
```

`lowerable_to_numeric_kernel_plan` means the candidate has enough semantic
metadata to describe numeric operations. It does not mean native code exists.

`interface_only` means the candidate is recorded to preserve the future backend
boundary but should not be benchmarked or presented as executable JIT work.

## Intended Consumers

Use the kernel plan for:

- runtime optimization track smoke checks
- native IDE Runtime/Inspector summaries
- coarse candidate size/cost inspection
- `eng-jit-bench-v1` interpreter baseline harness metadata
- future backend lowering tests

Do not use the kernel plan as:

- proof of runtime acceleration
- a replacement for `.engres`
- a public performance claim
- a stable AOT package contract
