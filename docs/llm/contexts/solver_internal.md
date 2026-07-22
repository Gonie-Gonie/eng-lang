# Solver Internal Context

Use this context only for solver implementation or solver documentation tasks.

Solver paths are implementation tracks that support semantic engineering
workflows. They should produce typed TimeSeries, residual evidence,
convergence/failure metadata, report/review artifacts, and IDE inspector data.

Do not generalize the current narrow solver implementations into:

- a general nonlinear solver platform
- broad DAE support
- production multi-domain component solving
- a Modelica/Simulink replacement
- an EnergyPlus replacement

Primary docs:

- `docs/internal/solver/README.md`
- `docs/current/main_internal_status.md`
- `docs/internal/solver/solver_centered_plan.md`
- `docs/internal/solver/generic_solver_completion_plan.md`
- `docs/current/feature_maturity_matrix.md`
