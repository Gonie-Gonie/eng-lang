# Measured vs Simulated

This internal fixture exercises a typed measured-vs-simulated path. Native
linear resampling materializes measured data on the simulated Time axis, and
that output then produces an RMSE metric, a validation result, a multi-series
PlotSpec, and reviewable artifacts.

The schemas also exercise the runtime data-quality surface:

```text
- weather_data.T_out and measured_data.T_zone use missing-value `error` policies
- both CSV inputs require monotonic DateTime indexes
- RoomThermal declares `T_out` as `TimeSeries[Time] of AbsoluteTemperature`
- simulate with-options are checked for TimeSeries input quantity, Time axis,
  duration timestep, and internal solver metadata
- `measured_on_sim` is an actual TimeSeries produced by `resample`, not an
  assumed external artifact
- artifact output records resampling status/counts and metric alignment status
```

What to inspect after `Run`:

```text
- sim.T_zone in result.engres as the actual emitted one-state trajectory
- measured_on_sim in the object store as a materialized seven-point TimeSeries
- measured_on_sim materialization status, reason, target count, and output count
- rmse_T with TemperatureDelta/K units
- validation status for the RMSE threshold
- time-alignment metadata linking measured_on_sim to its source and target
- resampled-measured/simulated two-series PlotSpec and SVG output
- solver method, timestep, step count, final state, and solver-boundary fields
```

Regression fixtures:

```text
- data/measured_zone_time_mismatch.csv: measured samples are spaced every 20 min,
  so linear resampling fills the simulated 10 min axis while artifacts retain
  the four exact timestamp matches
- data/measured_zone_missing.csv: one measured T_zone cell is empty, so the
  missing-value policy records a violation while native linear resampling and
  the run artifacts remain inspectable
```

Current limitation:

```text
- one-state thermal system
- fixed-step one-state ODE runner
- measured/weather CSV TimeSeries inputs only
- not public solver support
- not a general solver, calibration engine, DAE solver, adaptive solver, or
  multi-state simulation framework
```
