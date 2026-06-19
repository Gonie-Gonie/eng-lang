# B06 Nonlinear Solver

Focus: narrow source Newton residual solve and its residual/Jacobian/Newton-step
kernel planning metadata.

Run:

```bat
eng.exe jit-bench benchmarks\B06_nonlinear_solver\main.eng --iterations 1
```

Expected coverage is recorded in `expected.json` and checked by
`dev.bat jit-check`.
