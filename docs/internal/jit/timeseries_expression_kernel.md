# TimeSeries Expression Kernel

The current runtime has one supported table-to-TimeSeries expression kernel:

```eng
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
```

It lowers a typed CSV table into `TimeSeries[Time] of HeatRate` when these
conditions are met:

- the output binding name suggests heat rate, such as `Q_coil`;
- the expression references a promoted table mass-flow column such as
  `sensor.m_dot`;
- the expression references supply and return temperature columns;
- the expression contains a specific heat scalar such as `cp = 4180 J/kg/K`;
- the promoted schema has a DateTime index for the Time axis.

The compiler records this as a `timeseries_kernels` review section with:

- binding name;
- kernel kind: `table_heat_rate_from_mass_flow_cp_delta_t`;
- source table when it can be inferred from field references;
- axis, quantity, display unit, expression, operations, status, and line.

Runtime materialization computes points with:

```text
y = m_dot * cp * (T_return - T_supply)
```

The output display unit is currently W, with user-facing conversion handled by
plot/report/print/export surfaces.

## Deferred

The supported kernel is not a general expression engine. Deferred work includes:

- arbitrary table formulas;
- multi-table joins;
- general unit algebra across all TimeSeries expressions;
- user-defined kernels;
- package-level data-source abstractions.
