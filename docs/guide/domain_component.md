# Domain And Component Guide

This guide documents the current domain/component metadata surface.
It is an Internal solver-track surface: the compiler records domains, ports,
and connections and checks domain compatibility. It also emits an
equation/residual assembly seed for compatible connection sets. When multiple
compatible domain families appear in the same component graph, artifacts use
legacy field values such as `solver_preview.status = multi_domain_preview`;
those are machine-readable Internal metadata labels, not a physical
multi-domain component solve claim.

## Domain Declaration

```eng
domain Thermal package "eng.std.domains.thermal" version "0.1.0" {
    across T: AbsoluteTemperature [degC]
    through Q: HeatRate [kW]
    conservation sum(Q) = 0
}
```

| Syntax | Meaning |
|---|---|
| `domain Thermal` | Declares a user-defined domain named `Thermal`. |
| `package "..."` | Records package identity metadata for review/report/IDE/LSP consumers. |
| `version "..."` | Records domain contract version metadata. |
| `across T` | Records an across variable, such as potential/temperature. |
| `through Q` | Records a through variable, such as flow/heat rate. |
| `[degC]`, `[kW]` | Display units recorded in review metadata. |
| `conservation sum(Q) = 0` | Conservation contract recorded as metadata. |

Across/through variables use the existing quantity registry. Current examples
use `AbsoluteTemperature`, `HeatRate`, `Length`, and `MassFlowRate`.

Generic metadata is supported for domain references. A parameter may be written
as a single identifier, or as `<Kind> <Name>` when the package wants to preserve
both the semantic kind and the local parameter name:

```eng
domain Fluid[Medium M] package "eng.std.domains.fluid" version "0.1.0" {
    across height: Length [m]
    through m_dot: MassFlowRate [kg/s]
    conservation sum(m_dot) = 0
}

domain MechanicalNode[Frame F, Axis DOF] package "eng.std.domains.mechanical" version "0.1.0" {
    across x: Length [m]
    through P: MechanicalPower [W]
    conservation sum(P) = 0
}
```

`Medium`, `Frame`, and `Axis` are parameter kinds. `M`, `F`, and `DOF` are local
parameter names. Review/report/LSP metadata keeps `kind`, `name`, and `display`
for each entry in `type_parameters`.

Every user-defined domain must declare at least one `across` variable, at least
one `through` variable, and at least one `conservation` line. Missing contract
parts produce `E-DOMAIN-CONTRACT-001`, `E-DOMAIN-CONTRACT-002`, or
`E-DOMAIN-CONTRACT-003`. Domain variables must use known quantity kinds; unknown
quantity kinds produce `E-DOMAIN-VAR-001`.

## Component Ports

```text
component RoomBoundary {
    port heat: Thermal
    ua_seed = 0.5 kW/K
}

component SupplyPipe {
    port inlet: Fluid[Water]
    port outlet: Fluid[Water]
}
```

Each `port` names a component boundary and references a declared domain. If the
domain is missing, checking reports `E-PORT-DOMAIN-001`. A `name = expr` line
inside a component is recorded as component-local expression metadata; it is not
a top-level workflow binding and is not numerically solved.

Generic domain ports must provide the expected number of type arguments. For
example, `Fluid[Medium M]` expects `Fluid[Water]`, `Fluid[Air]`, or another
single argument at the port boundary. `MechanicalNode[Frame F, Axis DOF]`
expects two arguments such as `MechanicalNode[World, X]`. Plain `Fluid` reports
`E-PORT-DOMAIN-002`.

## Connections

```text
connect RoomBoundary.heat -> AmbientBoundary.heat
```

Connections use source-order metadata. Both endpoints must be written as
`Component.port`, and both ports must resolve to the same domain.

| Diagnostic | Trigger |
|---|---|
| `E-DOMAIN-CONTRACT-001` | Domain has no `across` variable. |
| `E-DOMAIN-CONTRACT-002` | Domain has no `through` variable. |
| `E-DOMAIN-CONTRACT-003` | Domain has no `conservation` line. |
| `E-DOMAIN-VAR-001` | Domain variable uses an unknown quantity kind. |
| `E-CONNECT-ENDPOINT-001` | Endpoint is not written as `Component.port`. |
| `E-CONNECT-UNKNOWN-PORT` | Endpoint does not resolve to a declared component port. |
| `E-CONNECT-DOMAIN-MISMATCH` | Both ports resolve, but their domains differ. |
| `E-CONNECT-MEDIUM-MISMATCH` | Same generic domain, but `Medium` arguments differ. |
| `E-CONNECT-FRAME-001` | Same generic domain, but `Frame` arguments differ. |
| `E-CONNECT-AXIS-001` | Same generic domain, but `Axis` arguments differ. |
| `E-CONNECT-DUPLICATE-001` | The same component-port pair is connected more than once, including reversed duplicates. |
| `E-DELAY-CALL-001` | Component-local delay call is not `delay(signal, duration)`. |
| `E-DELAY-SIGNAL-001` | Delay signal is not a known `port.variable`. |
| `E-DELAY-DURATION-001` | Delay duration is not a positive time value. |
| `E-PREDICTOR-CALL-001` | Component-local Predictor call is not `predictor(signal)` or `predict(signal)`. |
| `E-PREDICTOR-SIGNAL-001` | Predictor signal is not a known `port.variable`. |
| `E-EXTERNAL-BEHAVIOR-CALL-001` | Component-local external behavior call is not `external(signal)` or `adapter(signal)`. |
| `E-EXTERNAL-BEHAVIOR-SIGNAL-001` | External behavior signal is not a known `port.variable`. |
| `W-CONNECT-UNCONNECTED-PORT` | A resolved component port has no connection edge. |

Connection summaries are emitted in source order. They are not sorted by graph
topology because numeric graph solving is still deferred.

## Assembly Seed

Compatible connections are grouped into connection sets. For each set, the
compiler records Internal generated equation seeds:

- across variables generate equality equations, such as
  `RoomBoundary.heat.T eq AmbientBoundary.heat.T`;
- through variables generate conservation equations, such as
  `sum(RoomBoundary.heat.Q, AmbientBoundary.heat.Q) eq 0`;
- variables are classified as algebraic unknowns for the current seed;
- equation/unknown counts are recorded with an underdetermined or
  overdetermined diagnostic-code seed when the metadata is not balanced;
- residual graph metadata records residual names, dependencies, algebraic-loop
  candidates, Jacobian sparsity placeholders, and a solve-plan placeholder;
- `domain_plans` group generated constraints by instantiated domain, such as
  `Thermal`, `Fluid[Water]`, and `MechanicalNode[World, X]`;
- `solver_preview.status` is the current artifact field for identifying when
  one assembly contains more than one domain plan.

This is not a physical component graph solver. The generated equations are
review/report metadata for future assembly and solver work.

## Connection Constraint Check

`eng run` assembles generated connection equations into a residual graph and
then into a dense linear residual system when the graph is square. Non-square
graphs still get a numeric residual check. If the homogeneous constraints are
satisfied but there are fewer equations than unknowns, the result is marked
`constraint_satisfied_nonunique` with `E-ASSEMBLY-UNDERDETERMINED`.

The runtime result artifact writes this to
`typed_payload.component_solutions`. Runtime `report_spec.json` and
`report.html` also expose the updated assembly status and convergence metadata.
This path is useful for linear residual assembly, dense solver plumbing,
convergence/failure artifacts, and future solver integration, but it is not a
physical multi-domain solve.

The artifact also records explicit future-solver seeds:

- algebraic-only versus mixed state/algebraic classification;
- symbolic nonlinear residual seed status;
- DAE split seed status;
- delay/history buffer seed status, including whether delay calls are backed by
  the runtime delay-buffer seed but not yet integrated as a component solve;
- Predictor behavior contract and external adapter wrapper seed status;
- limitations: `not_full_dae`, `not_general_nonlinear`, `not_adaptive`,
  `not_production_multi_domain`, and `no_jit_speed_claim`.

Component-local `delay(signal, duration)` calls are checked for a known
`port.variable` signal and a positive time duration such as `5 s`.
Component-local `predictor(signal)`/`predict(signal)` and
`external(signal)`/`adapter(signal)` calls are also checked for a single known
`port.variable` signal. Full Predictor and external behavior contracts remain
runtime-wrapper seeds until behavior graph solving is wired into supported
solver paths.

## Artifact Surface

`eng check --review` writes domain/component information to `review.json`:

```bat
target\debug\eng.exe check examples\official\06_domain_port\main.eng --review
```

`eng run` also carries the same metadata into runtime report objects. Add
`--save-artifacts` for user-facing report files:

```bat
target\debug\eng.exe run examples\official\06_domain_port\main.eng --save-artifacts
```

The current domain/component artifact sections are:

```text
domain_summary
component_summary
connection_summary
assembly_summary
component_graph
```

They appear in `review.json` and `build/result/report_spec.json`; the same
domain/component information is summarized in `build/result/report.html`.
This is the domain/component artifact surface: the report shows which domains
exist, package/version metadata, generic type parameters, which component ports
reference them, port type arguments, and whether each connection is currently
`domain_compatible` or diagnostic-only.
The `assembly_summary` section shows connection sets, generated connection
equations, generated reasons, variable/equation counts, residual graph
dependencies, and solver plan placeholders. It also includes `domain_count`,
`domain_plans`, and `solver_preview` so report, IDE, and automation consumers can distinguish a
single-domain graph from a multi-domain metadata graph.
The `component_graph` section is a normalized graph JSON view with component
nodes, port nodes, connection edges, connection sets, domain labels,
behavior nodes for delay/Predictor/external expressions, medium/frame/axis
labels when present, and source spans for graph navigation.
The native IDE Assembly panel renders the same graph and lets connection,
port, component, and behavior rows jump back to their recorded source lines.
`component_summary.local_expressions` and
`assembly_summary.local_expression_count` record component-local `name = expr`
metadata without promoting it to the root runtime object store.
The runtime result also includes `component_solutions` with residual values,
normalized residuals, convergence status, solved linear variables when a square
system is available, zero-seed variables for skipped non-square graphs, and
failure/limitation artifacts. Runtime `report_spec.json` mirrors the same
details under `assembly_summary[].solver_result`.

The generated `report_spec.json` follows
[`docs/schemas/report_spec.schema.json`](../schemas/report_spec.schema.json), so
portable and native IDE releases can render the same metadata without
re-parsing source files.

## Official Examples

- `examples/official/06_domain_port/main.eng`
  shows compatible Thermal, `Fluid[Water]`, and
  `MechanicalNode[World, X]` domain connections with package/version metadata
  and structured generic parameter metadata. Its assembly artifacts report
  three domain plans and `solver_preview.status = multi_domain_preview`.
- `examples/official/21_thermal_component_assembly/main.eng`
  focuses on one Thermal connection set with component-local boundary RHS
  equations. Its artifacts record generated connection equations, a square
  residual graph, dense linear solve status, solved variables, and residual
  values without claiming a production multi-domain component solver.
- `examples/05_error_messages/port_domain_mismatch.eng`
  intentionally connects a Thermal port to a Fluid port and should report
  `E-CONNECT-DOMAIN-MISMATCH` with a non-zero check exit.
- `examples/05_error_messages/medium_mismatch.eng`,
  `frame_mismatch.eng`, and `axis_mismatch.eng`
  intentionally connect generic domain ports with incompatible metadata
  arguments.
- `examples/05_error_messages/duplicate_connection.eng`
  intentionally repeats the same connection edge in reverse order and should
  report `E-CONNECT-DUPLICATE-001`.
- `examples/05_error_messages/generic_domain_arity.eng`
  intentionally omits the required generic domain argument.
- `examples/05_error_messages/domain_missing_across.eng`,
  `domain_missing_through.eng`, `domain_missing_conservation.eng`, and
  `domain_unknown_quantity.eng`
  intentionally violate user-defined domain contract rules.

## Support Boundary

Current:

- parser and semantic metadata;
- domain package/version metadata;
- structured generic domain parameter and port argument metadata;
- domain contract diagnostics;
- domain variable quantity/unit metadata;
- port domain validation;
- connection compatibility diagnostics;
- duplicate connection diagnostics and unconnected port warnings;
- medium/frame/axis metadata compatibility diagnostics;
- Internal connection-set assembly metadata;
- generated connection-equation and residual graph artifacts;
- homogeneous connection-constraint residual evaluation artifact;
- component behavior-node graph artifacts for delay/Predictor/external
  expressions;
- multi-domain assembly metadata with domain plans, future nonlinear/
  DAE/delay/Predictor/adapter seed statuses, and explicit limitations;
- review JSON output;
- report spec and HTML report sections;
- native IDE Domain Graph inspector;
- LSP snapshot/completion/hover metadata for domains, variables, ports, and
  connections, including port type/base-domain and medium/frame/axis labels;
- IDE/VS Code keyword and snippet completion.

Deferred:

- physical component graph solving with boundary conditions and component
  behavior equations;
- production multi-domain numerical solving;
- package registries;
- package dependency resolution;
- numeric enforcement of conservation contracts.
