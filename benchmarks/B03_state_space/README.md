# B03 State Space

Focus: continuous state-space RHS and explicit-Euler solver-step kernel planning
coverage. The runtime simulation still uses the normal runtime path.

Run:

```bat
eng.exe jit-bench benchmarks\B03_state_space\main.eng --iterations 1
```

Expected coverage is recorded in `expected.json` and checked by
`dev.bat jit-check`.
