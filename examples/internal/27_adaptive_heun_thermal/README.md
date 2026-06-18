# Internal 27 - Adaptive Heun Thermal

This internal fixture exercises the supported one-state thermal
`solver = adaptive_heun` simulation path:

- explicit `TimeSeries[Time]` input binding for `T_out`
- fixed report/output TimeGrid from `timestep` plus `duration`
- numeric `tolerance` option
- adaptive Heun/Euler internal substep diagnostics in runtime/report artifacts

It is not a general adaptive equation-system solver. It exists to keep the
implemented one-state thermal adaptive path covered by the development and IDE
smoke gates.
