# Kernel Plan Reference

`eng.exe jit-plan <file.eng>` emits experimental hot-kernel metadata for the
v1.4 JIT-start path. The output is a planning artifact only. It does not mean
native code has been generated, selected, cached, or executed.

The current format marker is:

```json
{
  "format": "eng-kernel-plan-v1"
}
```

## Stability Policy

`eng-kernel-plan-v1` is experimental while v1.4 is not release-supported.
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
  "candidate_count": 3,
  "candidates": []
}
```

`backend = "interpreter-fallback"` means the normal EngLang runtime still owns
execution. Current JIT planning makes no speedup claim.

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
  ]
}
```

`line` is one-based and points at the source construct that produced the
candidate.

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

- v1.4 JIT gate smoke checks
- native IDE Runtime/Inspector summaries
- future benchmark harness selection
- future backend lowering tests

Do not use the kernel plan as:

- proof of runtime acceleration
- a replacement for `.engres`
- a public performance claim
- a stable AOT package contract
