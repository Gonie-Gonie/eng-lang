# Solver Track Docs

This directory is the entry point for solver implementation context.
It is not product identity and it is not public package scope unless a current
status document says so.

EngLang is a semantic engineering workflow language. Solver paths matter when
they produce typed TimeSeries, residual/convergence evidence, and reviewable
artifacts that support engineering validation.

## Read Order

1. [Current project status](../../current/status.md)
2. [Feature maturity matrix](../../current/feature_maturity_matrix.md)
3. [Development tracks](../../current/tracks.md)
4. [Solver-centered implementation plan](solver_centered_plan.md)
5. [Generic solver completion plan](generic_solver_completion_plan.md)

## Scope Boundary

Use these labels consistently:

- `Public package`: documented workflow available in the current portable
  package.
- `Supported`: narrow, documented, tested main behavior with explicit limits.
- `Internal`: implementation seed or regression fixture, not public support.
- `Planned`: future work.

Do not present one-state thermal, source-equation ODE, state-space, Newton,
DAE, behavior-node, or component residual smokes as a broad solver platform.

## Current Positioning

Solver work should be described as:

- a typed TimeSeries producer
- a source of residual and convergence evidence
- a way to validate measured-vs-simulated workflows
- an implementation track for future engineering computation

Solver work should not be described as:

- the primary identity of EngLang
- a Modelica/Simulink replacement
- a production multi-domain simulator
- a general nonlinear/DAE solver release
