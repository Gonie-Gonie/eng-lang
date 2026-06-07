# Domain And Component Guide

This guide documents the current experimental v2.0 domain/component surface.
It is metadata-first: the compiler records domains, ports, and connections and
checks domain compatibility, but it does not solve a multi-domain component
graph yet.

## Domain Declaration

```eng
domain Thermal {
    across T: AbsoluteTemperature [degC]
    through Q: HeatRate [kW]
    conservation sum(Q) = 0
}
```

| Syntax | Meaning |
|---|---|
| `domain Thermal` | Declares a user-defined domain named `Thermal`. |
| `across T` | Records an across variable, such as potential/temperature. |
| `through Q` | Records a through variable, such as flow/heat rate. |
| `[degC]`, `[kW]` | Display units recorded in review metadata. |
| `conservation sum(Q) = 0` | Conservation contract recorded as metadata. |

Across/through variables use the existing quantity registry. Current examples
use `AbsoluteTemperature`, `HeatRate`, `Length`, and `MassFlowRate`.

## Component Ports

```text
component RoomBoundary {
    port heat: Thermal
}

component AmbientBoundary {
    port heat: Thermal
}
```

Each `port` names a component boundary and references a declared domain. If the
domain is missing, checking reports `E-PORT-DOMAIN-001`.

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
the report shows which domains exist, which component ports reference them, and
whether each connection is currently `domain_compatible` or diagnostic-only.

The generated `report_spec.json` follows
[`docs/schemas/report_spec.schema.json`](../schemas/report_spec.schema.json), so
portable and native IDE releases can render the same metadata without
re-parsing source files.

## Official Examples

- `examples/official/06_domain_port/main.eng`
  shows compatible Thermal and Fluid domain connections.
- `examples/05_error_messages/port_domain_mismatch.eng`
  intentionally connects a Thermal port to a Fluid port and should report
  `E-CONNECT-DOMAIN-001` with a non-zero check exit.

## Support Boundary

Current:

- parser and semantic metadata;
- domain variable quantity/unit metadata;
- port domain validation;
- connection compatibility diagnostics;
- review JSON output;
- IDE/LSP/VS Code keyword and snippet completion.

Deferred:

- graph solving;
- generic domains such as `Fluid[Medium]`;
- frame/axis compatibility;
- domain package versioning and registries.
