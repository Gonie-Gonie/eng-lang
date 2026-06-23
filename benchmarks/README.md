# EngLang Benchmark Strategy

EngLang should be evaluated first as a semantic engineering workflow language.
Runtime and solver timing are useful internal signals, but they are not the
primary product benchmark.

## Semantic Benchmarks

These benchmarks should measure whether EngLang makes engineering computation
easier to review and less error-prone.

| Benchmark | What it measures |
|---|---|
| Unit/schema injected error detection | Whether incorrect unit, quantity, or schema assumptions are caught before artifacts look valid. |
| TimeSeries alignment error detection | Whether axis and time-alignment mistakes are made visible in diagnostics or review artifacts. |
| LLM-generated code correction time | How quickly an agent or reviewer can repair generated EngLang code using diagnostics and artifacts. |
| Reviewer validation time | How quickly a human can confirm a result from variables, units, plots, reports, and provenance. |
| Report/provenance completeness | Whether generated artifacts explain inputs, outputs, side effects, and validation decisions. |
| Manual unit conversion reduction | Whether the workflow avoids ad hoc unit conversions in user code. |

The benchmark harness for these semantic cases is still planned. New benchmark
cases should prefer deterministic local input data, checked expected artifacts,
and reviewable failure cases.

## Internal Solver / Runtime Benchmarks

The existing catalog is used by the runtime optimization track. It is
intentionally small and deterministic: each case has local input data, an
EngLang source file, expected target metadata, timing policy notes, and a
correctness check wired through `dev.bat jit-check`.

The benchmark harness is still:

```bat
eng.exe jit-bench <file.eng> --iterations N
```

It measures the normal interpreter/runtime path with artifact generation
enabled and records kernel-planning coverage. It does not claim native JIT
speedups.

## Catalog Cases

| Case | Focus | Source |
|---|---|---|
| B01_csv_heat_rate | CSV promotion, TimeSeries arithmetic, integration | `B01_csv_heat_rate/main.eng` |
| B02_timeseries_fusion | Multi-statistics TimeSeries fusion | `B02_timeseries_fusion/main.eng` |
| B03_state_space | Continuous state-space RHS and solver-step candidates | `B03_state_space/main.eng` |
| B04_residual_eval | Small Thermal residual/Jacobian evaluator candidates | `B04_residual_eval/main.eng` |
| B05_component_solver | Thermal/Fluid component residual graph solve shape | `B05_component_solver/main.eng` |
| B06_nonlinear_solver | Narrow nonlinear Newton residual solve shape | `B06_nonlinear_solver/main.eng` |

Run the catalog check:

```bat
.\dev.bat jit-check
```

The check runs each `main.eng` through `jit-bench --iterations 1`, verifies the
expected benchmark targets, checks local input data exists, confirms measured
interpreter timing fields are present, and verifies that runtime artifacts were
generated.

## Claim Boundary

Do not use the internal benchmark catalog as a solver-performance claim.
Until native code generation and benchmark parity gates exist, benchmark output
is coverage metadata for the optimization track.
