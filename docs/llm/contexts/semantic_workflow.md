# Semantic Workflow Context

Use this context for README, docs, examples, package, and user-guide work.

EngLang is a semantic engineering workflow language. The public story should
lead with units, quantities, schemas, axes, TimeSeries, provenance, reports,
review artifacts, and IDE inspection.

Solver language should be supporting language:

- good: "scoped simulation produces typed TimeSeries"
- good: "residual and convergence artifacts are reviewable"
- avoid: "general solver platform"
- avoid: "Modelica/Simulink replacement"
- avoid: "production multi-domain simulator"

When editing public docs, keep the first screen focused on:

1. typed data boundary
2. unit/quantity-aware TimeSeries calculation
3. plot/report/review artifacts
4. measured-vs-simulated validation
5. explicit side effects
6. native tester IDE and portable package
