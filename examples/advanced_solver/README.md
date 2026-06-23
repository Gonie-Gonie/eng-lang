# Advanced Solver Smoke Fixtures

These examples are implementation and regression fixtures for narrow
solver/source/component paths. They are not the first-user walkthrough and they
are not broad solver platform claims.

Use these files when working on solver internals, report/review solver
artifacts, IDE solver panels, or regression gates.

```text
20_multi_state_thermal
21_state_space_discrete
22_state_space_continuous
23_thermal_component_assembly
24_linear_algebraic_thermal_node
25_fixed_point_loop
26_dynamic_component_room
27_nonlinear_algebraic
28_small_dae
29_delay_component_solver
30_predictor_component_solver
31_external_behavior_solver
32_small_thermal_fluid_loop
33_unit_parameterized_wall
34_three_state_source_ode
```

The useful output from these examples is typed TimeSeries, residual evidence,
convergence/failure metadata, report/review artifacts, and IDE inspection data.

Do not describe this directory as:

- a general nonlinear solver suite
- broad DAE support
- production component graph solving
- production multi-domain simulation
- a Modelica/Simulink or EnergyPlus replacement
