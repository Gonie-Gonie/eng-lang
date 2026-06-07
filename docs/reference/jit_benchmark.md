# JIT Benchmark Harness Reference

`eng.exe jit-bench <file.eng>` emits an experimental `eng-jit-bench-v1`
artifact. It is a v1.4 planning harness, not a performance claim.

The current harness measures only the normal interpreter/runtime path. The JIT
side is present in the JSON shape as `status = "not_available"` so future native
backend timing can be added without changing the consumer boundary.

## Top-Level Shape

```json
{
  "format": "eng-jit-bench-v1",
  "source_path": "examples/official/01_csv_plot/main.eng",
  "iterations_requested": 1,
  "comparison_policy": "no-speedup-claim",
  "kernel_plan": {},
  "interpreter": {},
  "jit": {},
  "notes": []
}
```

`kernel_plan` is the same `eng-kernel-plan-v1` object documented in
[Kernel plan reference](kernel_plan.md).

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

`eng-jit-bench-v1` is experimental while v1.4 is not release-supported.

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
