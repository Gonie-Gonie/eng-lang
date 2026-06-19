# B02 TimeSeries Fusion

Focus: multi-statistics TimeSeries fusion over the same checked CSV shape used
by the official heat-rate workflow.

Run:

```bat
eng.exe jit-bench benchmarks\B02_timeseries_fusion\main.eng --iterations 1
```

Expected coverage is recorded in `expected.json` and checked by
`dev.bat jit-check`.
