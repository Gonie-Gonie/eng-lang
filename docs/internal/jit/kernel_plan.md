# Kernel Plan Reference

`eng.exe jit-plan <file.eng>` emits internal hot-kernel metadata for the
runtime optimization track. The output includes per-candidate interpreter
executor status, but it does not mean native code has been generated, selected,
cached, or executed.

The current format marker is:

```json
{
  "format": "eng-kernel-plan-v1"
}
```

## Stability Policy

`eng-kernel-plan-v1` is internal and not public release-supported.
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

## Report And IDE Surface

`report_spec.json` embeds a `kernel_plan` object using the same top-level
shape, candidate fields, executor status, and fallback reason described here.
`report.html` renders that data as a Runtime Optimization Kernel Plan table,
and the portable native IDE exposes it in the Kernel inspector panel. These surfaces are
for inspection only; they are not evidence of native execution or acceleration.

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
  },
  "executor": {
    "backend": "interpreter-fallback",
    "status": "interpreter_supported",
    "fallback_reason": "candidate can execute through the interpreter kernel IR when runtime inputs are supplied"
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
as scalar residual kernels that do not operate over a row axis.

`operation_count` is an operation-class count used for planning and inspection.
It is not a floating-point operation count and must not be used for performance
claims.

## Executor Shape

`executor` records whether the internal interpreter kernel IR can execute the
candidate when runtime inputs are supplied. It is still not a native backend.

```json
{
  "backend": "interpreter-fallback",
  "status": "interpreter_supported",
  "fallback_reason": "candidate can execute through the interpreter kernel IR when runtime inputs are supplied"
}
```

Known statuses:

```text
interpreter_supported
fallback_metadata_only
```

`fallback_metadata_only` remains available for a candidate whose planning
metadata cannot be reconstructed as executable interpreter IR. Current
source-system residual candidates are emitted only after executable lowering
succeeds.

## Kernel IR

The internal interpreter executor uses `eng-kernel-ir-v1` with explicit
instructions for loading TimeSeries inputs, scalar inputs, constants, binary
arithmetic, unary dimensionless functions, series/scalar stores, TimeSeries
statistics reductions, and trapezoid integration. This IR currently supports
correctness tests for element-wise arithmetic, statistics, integration,
source-system and component residual evaluation, continuous state-space A/B RHS
evaluation, finite-difference Jacobian kernels, and Newton solver-step kernels.
It is not a native code format and is not part of the public stable API.

## Candidate Kinds

```text
timeseries_arithmetic
timeseries_integrate
statistics_fusion
system_residual
system_residual_jacobian
system_newton_step
component_residual_graph
component_residual_jacobian
component_newton_step
state_space_rhs
state_space_solver_step
```

`system_residual` executes compiler-normalized source-system residuals over an
explicit scalar input layout. Parameters, inputs, states, and outputs retain
compiler declaration order; referenced `der(state)` values and `t` or `time`
are appended as explicit runtime context when required. Registered unit
literals retain the runtime source-residual numeric convention.
`system_residual_jacobian` perturbs only state/output input indices while
holding parameter and input context fixed. `system_newton_step` executes one
dense Newton update when that unknown layout is non-empty and square and no
derivative input is required. These are executable interpreter kernels, not an
integrated solver loop or a native backend.
`component_residual_jacobian` records finite-difference Jacobian evaluation for
square component residual graphs by repeatedly executing the scalar residual
interpreter kernel.
`component_newton_step` records a single dense Newton update using a residual
vector and dense Jacobian for square component residual graphs. It is not an
integrated nonlinear solver loop.
`state_space_rhs` covers the continuous `der(x) eq A * x + B * u` scalar RHS
kernel for checked state-space A/B operators. The fixed-step solver loop still
runs through the normal runtime path.

## Lowering Status

```text
lowerable_to_numeric_kernel_plan
interface_only
```

`lowerable_to_numeric_kernel_plan` means the candidate has enough semantic
metadata to describe numeric operations. Candidate `executor.status` must still
be checked to know whether the current interpreter IR can execute that specific
candidate. It does not mean native code exists.

`interface_only` is retained as an internal compatibility status for planning
records without executable IR. Such candidates must not be benchmarked or
presented as executable JIT work.

## Intended Consumers

Use the kernel plan for:

- runtime optimization track smoke checks
- native IDE Runtime/Inspector summaries
- coarse candidate size/cost inspection
- `eng-jit-bench-v1` interpreter baseline harness metadata
- interpreter kernel correctness tests and future backend lowering tests

Do not use the kernel plan as:

- proof of runtime acceleration
- a replacement for `.engres`
- a public performance claim
- a stable AOT package contract
