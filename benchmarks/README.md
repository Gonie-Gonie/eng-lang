# EngLang Solver Benchmark Catalog

This directory contains the solver-centered benchmark catalog used by the
runtime optimization track. The catalog is intentionally small and deterministic:
each case has local input data, an EngLang source file, expected target metadata,
timing policy notes, and a correctness check wired through `dev.bat jit-check`.

The benchmark harness is still `eng.exe jit-bench <file.eng> --iterations N`.
It measures the normal interpreter/runtime path with artifact generation enabled
and records kernel-planning coverage. It does not claim native JIT speedups.

## Cases

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
