# JIT Benchmark Harness Reference

`eng.exe jit-bench <file.eng>` emits an experimental `eng-jit-bench-v1`
artifact. It is a runtime optimization track planning harness, not a
performance claim.

The current harness measures only the normal interpreter/runtime path. It also
records deterministic sample executions for lowerable interpreter kernel IR
candidates so consumers can see which candidates have executable fallback
coverage. These samples are correctness evidence, not timing comparisons. The
JIT side is present in the JSON shape as `status = "not_available"` so future
native backend timing can be added without changing the consumer boundary.

## Top-Level Shape

```json
{
  "format": "eng-jit-bench-v1",
  "source_path": "examples/official/01_csv_plot/main.eng",
  "iterations_requested": 1,
  "comparison_policy": "no-speedup-claim",
  "kernel_plan": {},
  "benchmark_targets": [],
  "kernel_executor_samples": [],
  "interpreter": {},
  "jit": {},
  "notes": []
}
```

`kernel_plan` is the same `eng-kernel-plan-v1` object documented in
[Kernel plan reference](kernel_plan.md).

## Benchmark Targets

`benchmark_targets` records which solver-centered benchmark targets were
observed in the current source's kernel plan. It is target-coverage metadata,
not a timing comparison.

```json
[
  {
    "name": "csv_heat_rate_workflow",
    "status": "covered_by_current_source",
    "candidate_count": 3,
    "candidates": [
      "timeseries_arithmetic:Q_coil",
      "statistics_fusion:summary:Q_coil",
      "timeseries_integrate:E_coil"
    ],
    "note": "covers checked TimeSeries arithmetic/statistics/integration candidates when present"
  }
]
```

Known statuses:

```text
covered_by_current_source
interface_only
metadata_observed
not_observed_for_source
```

Current target names are `csv_heat_rate_workflow`,
`multi_statistics_fusion`, `residual_evaluation`,
`component_graph_solver_small_case`, and `state_space_simulation`.
Targets that are not present in the input source remain
`not_observed_for_source`.
`metadata_observed` means the source exposes checked metadata for the target,
but the current kernel plan has not selected an executable kernel for that
target.
For continuous state-space sources, `state_space_simulation` is
`covered_by_current_source` when the plan includes an interpreter-supported
`state_space_rhs` candidate. This covers the A/B RHS kernel only; fixed-step
simulation still uses the normal runtime solver path and no native timing is
reported.
For square component assemblies, `residual_evaluation` may list both
`component_residual_graph` and `component_residual_jacobian` candidates.
`component_graph_solver_small_case` may additionally list
`component_newton_step`. This records residual/Jacobian/single-step kernel
coverage, not a production component-graph solver claim.

## Benchmark Catalog

The repository includes a lightweight benchmark catalog under `benchmarks/`:

```text
benchmarks/
  B01_csv_heat_rate/
  B02_timeseries_fusion/
  B03_state_space/
  B04_residual_eval/
  B05_component_solver/
  B06_nonlinear_solver/
```

Each case contains local input data, `main.eng`, `expected.json`, and a short
README. `expected.json` records the expected benchmark target coverage,
interpreter executor samples, correctness fragments in the generated
`result.engres`, and the timing policy. `dev.bat jit-check` runs every catalog
case with `--iterations 1`, verifies the expected coverage, checks that input
data exists, and confirms `result.engres`, `report.html`, `report_spec.json`,
and `review.json` were generated.

The catalog is an optimization-track evidence set. It is not a native backend
performance suite while `jit.status` is `not_available`.

## Kernel Executor Samples

`kernel_executor_samples` records deterministic sample executions for
lowerable candidates using the interpreter kernel executor. Inputs are synthetic
and shape-oriented; they do not represent source data and must not be used as
benchmark timings.

```json
[
  {
    "candidate": "timeseries_integrate:E_coil",
    "kind": "timeseries_integrate",
    "status": "executed",
    "backend": "interpreter-fallback",
    "fallback_reason": null,
    "series_input_count": 1,
    "scalar_input_count": 0,
    "output_count": 1,
    "outputs": [
      {
        "kind": "scalar",
        "value": 900
      }
    ]
  }
]
```

Known statuses:

```text
executed
failed
```

Failures include `failure_code` and `failure_message` and indicate a sample
execution problem, not a normal runtime failure for the source program.

## Interpreter Shape

```json
{
  "status": "measured",
  "runs": [
    {
      "iteration": 1,
      "elapsed_ms": 12.345,
      "result_path": "build/jit-bench/iter-000/result/result.engres"
    }
  ],
  "summary": {
    "average_ms": 12.345,
    "min_ms": 12.345,
    "max_ms": 12.345,
    "total_ms": 12.345
  }
}
```

Interpreter timings are local smoke measurements. They include the current
runtime path and artifact generation behavior used by `eng.exe run`.

## JIT Shape

```json
{
  "status": "not_available",
  "backend": "interpreter-fallback",
  "runs": [],
  "summary": null
}
```

This means no native backend has been selected, compiled, cached, or executed.
Do not calculate or publish speedups from this artifact while `jit.status` is
`not_available`.

Use `--backend native-preview` only to test selection metadata:

```bat
eng.exe jit-bench examples\official\01_csv_plot\main.eng --iterations 1 --backend native-preview
```

The resulting `kernel_plan.backend_selection.status` remains `not_available`
and `jit.status` remains `not_available`.

## Stability Policy

`eng-jit-bench-v1` is experimental and not public release-supported.

- Existing top-level keys keep their current type.
- New optional keys may be added.
- Consumers must ignore unknown keys.
- Changing the meaning of `comparison_policy`, `interpreter.status`, or
  `jit.status` requires a new format marker.

## Verification

```bat
eng.exe jit-bench examples\official\01_csv_plot\main.eng --iterations 1
.\dev.bat jit-check
```
