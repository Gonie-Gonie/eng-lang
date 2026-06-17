# JIT Benchmark Harness Reference

`eng.exe jit-bench <file.eng>` emits an experimental `eng-jit-bench-v1`
artifact. It is a runtime optimization track planning harness, not a
performance claim.

The current harness measures only the normal interpreter/runtime path. The
`eng_jit` crate also has an internal interpreter kernel IR/executor for
correctness tests, but `eng.exe jit-bench` does not report speedups from it.
The JIT side is present in the JSON shape as `status = "not_available"` so
future native backend timing can be added without changing the consumer
boundary.

## Top-Level Shape

```json
{
  "format": "eng-jit-bench-v1",
  "source_path": "examples/official/01_csv_plot/main.eng",
  "iterations_requested": 1,
  "comparison_policy": "no-speedup-claim",
  "kernel_plan": {},
  "benchmark_targets": [],
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
