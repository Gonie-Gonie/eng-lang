# Measured vs Simulated

This official example closes the v3 roadmap fusion path: typed measured data
and typed simulation output meet as TimeSeries, then produce an RMSE metric, a
validation result, a multi-series PlotSpec, and reviewable artifacts.

The schemas also exercise the runtime data-quality surface:

```text
- weather_data.T_out and measured_data.T_zone use missing-value `error` policies
- both CSV inputs require monotonic DateTime indexes
- RoomThermal declares `T_out` as `TimeSeries[Time] of AbsoluteTemperature`
- simulate with-options are checked for TimeSeries input quantity, Time axis,
  duration timestep, and supported solver metadata
- artifact output records metric sample counts and TimeSeries alignment status
```

Regression fixtures:

```text
- data/measured_zone_time_mismatch.csv: measured samples are spaced every 20 min,
  so the measured/simulated TimeSeries alignment is recorded as partial overlap
- data/measured_zone_missing.csv: one measured T_zone cell is empty, so the
  missing-value policy records a violation while the run artifacts remain
  inspectable
```

Current limitation:

```text
- one-state thermal system
- fixed-step one-state ODE runner
- measured/weather CSV TimeSeries inputs only
- not a general solver, calibration engine, DAE solver, adaptive solver, or
  multi-state simulation framework
```
