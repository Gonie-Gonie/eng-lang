# Measured vs Simulated

This official example closes the v3 roadmap fusion path: typed measured data
and typed simulation output meet as TimeSeries, then produce an RMSE metric, a
validation result, a multi-series PlotSpec, and reviewable artifacts.

Current limitation:

```text
- one-state thermal system
- fixed-step preview ODE runner
- measured/weather CSV TimeSeries inputs only
- not a general solver, calibration engine, DAE solver, adaptive solver, or
  multi-state simulation framework
```
