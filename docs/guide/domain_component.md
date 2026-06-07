# Domain And Component Guide

This guide documents the current experimental v2.0 domain/component surface.
It is metadata-first: the compiler records domains, ports, and connections and
checks domain compatibility, but it does not solve a multi-domain component
graph yet.

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

Generic metadata is supported for domain references:

```eng
domain Fluid[Medium] package "eng.std.domains.fluid" version "0.1.0" {
    across height: Length [m]
    through m_dot: MassFlowRate [kg/s]
    conservation sum(m_dot) = 0
}
```

`Medium`, `Frame`, and `Axis` are metadata parameters. They are checked at
connection time but do not create a numeric solver yet.

## Component Ports

```text
component RoomBoundary {
    port heat: Thermal
}

component SupplyPipe {
    port inlet: Fluid[Water]
    port outlet: Fluid[Water]
}
```

Each `port` names a component boundary and references a declared domain. If the
domain is missing, checking reports `E-PORT-DOMAIN-001`.

Generic domain ports must provide the expected number of type arguments. For
example, `Fluid[Medium]` expects `Fluid[Water]`, `Fluid[Air]`, or another
single argument at the port boundary; plain `Fluid` reports
`E-PORT-DOMAIN-002`.

## Connections

```text
connect RoomBoundary.heat -> AmbientBoundary.heat
```

Connections use source-order metadata. Both endpoints must be written as
`Component.port`, and both ports must resolve to the same domain.

| Diagnostic | Trigger |
|---|---|
| `E-CONNECT-ENDPOINT-001` | Endpoint is not written as `Component.port`. |
| `E-CONNECT-PORT-001` | Endpoint does not resolve to a declared component port. |
| `E-CONNECT-DOMAIN-001` | Both ports resolve, but their domains differ. |
| `E-CONNECT-MEDIUM-001` | Same generic domain, but `Medium` arguments differ. |
| `E-CONNECT-FRAME-001` | Same generic domain, but `Frame` arguments differ. |
| `E-CONNECT-AXIS-001` | Same generic domain, but `Axis` arguments differ. |

Connection summaries are emitted in source order. They are not sorted by graph
topology because numeric graph solving is still deferred.

## Artifact Surface

`eng check --review` writes domain/component information to `review.json`:

```bat
target\debug\eng.exe check examples\official\06_domain_port\main.eng --review
```

`eng run` also carries the same metadata into the user-facing report artifacts:

```bat
target\debug\eng.exe run examples\official\06_domain_port\main.eng --entry main
```

The current domain/component artifact sections are:

```text
domain_summary
component_summary
connection_summary
```

They appear in `review.json`, `build/result/report_spec.json`, and
`build/result/report.html`. This is the v2.0 preview surface for user testing:
the report shows which domains exist, package/version metadata, generic type
parameters, which component ports reference them, port type arguments, and
whether each connection is currently `domain_compatible` or diagnostic-only.

The generated `report_spec.json` follows
[`docs/schemas/report_spec.schema.json`](../schemas/report_spec.schema.json), so
portable and native IDE releases can render the same metadata without
re-parsing source files.

## Official Examples

- `examples/official/06_domain_port/main.eng`
  shows compatible Thermal and `Fluid[Water]` domain connections with package
  and version metadata.
- `examples/05_error_messages/port_domain_mismatch.eng`
  intentionally connects a Thermal port to a Fluid port and should report
  `E-CONNECT-DOMAIN-001` with a non-zero check exit.
- `examples/05_error_messages/medium_mismatch.eng`,
  `frame_mismatch.eng`, and `axis_mismatch.eng`
  intentionally connect generic domain ports with incompatible metadata
  arguments.
- `examples/05_error_messages/generic_domain_arity.eng`
  intentionally omits the required generic domain argument.

## Support Boundary

Current:

- parser and semantic metadata;
- domain package/version metadata;
- generic domain parameter and port argument metadata;
- domain variable quantity/unit metadata;
- port domain validation;
- connection compatibility diagnostics;
- medium/frame/axis metadata compatibility diagnostics;
- review JSON output;
- report spec and HTML report sections;
- native IDE Domain Graph inspector;
- LSP snapshot/completion/hover metadata for domains, variables, ports, and
  connections;
- IDE/VS Code keyword and snippet completion.

Deferred:

- graph solving;
- package registries;
- package dependency resolution;
- numeric enforcement of conservation contracts.
