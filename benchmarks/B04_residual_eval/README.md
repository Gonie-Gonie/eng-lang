# B04 Residual Evaluation

Focus: small Thermal component residual graph evaluation and finite-difference
Jacobian planning.

Run:

```bat
eng.exe jit-bench benchmarks\B04_residual_eval\main.eng --iterations 1
```

Expected coverage is recorded in `expected.json` and checked by
`dev.bat jit-check`.
