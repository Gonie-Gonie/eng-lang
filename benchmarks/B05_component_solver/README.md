# B05 Component Solver

Focus: constrained Thermal/Fluid[Water] component residual graph solve shape,
including residual, Jacobian, and one-step Newton kernel planning metadata.

Run:

```bat
eng.exe jit-bench benchmarks\B05_component_solver\main.eng --iterations 1
```

Expected coverage is recorded in `expected.json` and checked by
`dev.bat jit-check`.
