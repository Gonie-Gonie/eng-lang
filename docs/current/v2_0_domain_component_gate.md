# v2.0 Domain/Component Gate

This page tracks the v2.0 path from data/report workflows toward an engineering
platform with user-defined domains, component ports, connection diagnostics, and
eventual package ecosystems.

The current support boundary is metadata-first. The compiler recognizes domain,
component, port, and connection declarations; validates port domain references;
and records review/report metadata. It does not run a multi-domain numeric
simulation yet.

## Current Scope On Main

- `domain <Name> { ... }` declares a user-defined domain.
- `domain <Name>[Kind LocalName] package "package.id" version "semver-ish" { ... }`
  records structured generic type-parameter, package, and version metadata.
- Single-name parameters such as `[Medium]` remain accepted; typed parameters
  such as `[Medium M]` preserve `kind`, `name`, and `display` metadata.
- `across <name>: <Quantity> [unit]` records across variables.
- `through <name>: <Quantity> [unit]` records through variables.
- `conservation <text>` records the domain conservation contract as metadata.
- Domains missing an `across`, `through`, or `conservation` contract part
  produce `E-DOMAIN-CONTRACT-001`, `E-DOMAIN-CONTRACT-002`, or
  `E-DOMAIN-CONTRACT-003`.
- Domain variables with unknown quantity kinds produce `E-DOMAIN-VAR-001`.
- `component <Name> { port <name>: <Domain> }` declares component ports.
- `port <name>: <Domain>[Argument]` instantiates a generic domain reference,
  such as `Fluid[Water]`.
- `connect Component.port -> Other.port` records a connection.
- Port declarations referencing unknown domains produce `E-PORT-DOMAIN-001`.
- Ports with the wrong number of generic domain arguments produce
  `E-PORT-DOMAIN-002`.
- Connections between ports of different domains produce
  `E-CONNECT-DOMAIN-001`.
- Connections between the same generic domain with different `Medium`, `Frame`,
  or `Axis` arguments produce `E-CONNECT-MEDIUM-001`,
  `E-CONNECT-FRAME-001`, or `E-CONNECT-AXIS-001`.
- Malformed connection endpoints produce `E-CONNECT-ENDPOINT-001`.
- Unresolved component ports produce `E-CONNECT-PORT-001`.
- Connection summaries are emitted in source order, not graph-topology order.
- `review.json` exposes `domain_summary`, `component_summary`, and
  `connection_summary`.
- `report_spec.json` and `report.html` expose the same domain/component surface
  so packaged runs and native IDE previews can show the declarations.
- The native IDE Inspector shows a Domain Graph section with domains,
  variables, conservations, component ports, and connection status.
- The native IDE smoke command verifies that the official domain example
  produces non-empty domain/component/connection metadata.
- LSP snapshots and hovers expose domain, domain-variable, conservation,
  component, port, and connection metadata with `kind`/`status` fields.
- LSP completions include domain names, domain variables, component names, and
  `Component.port` labels.
- The VS Code preview exposes domain/component keywords and snippets for user
  testing.

## Official Fixtures

- `examples/official/06_domain_port/main.eng`
  - declares Thermal and Fluid domains;
  - records package/version metadata;
  - declares `Fluid[Medium M]` and instantiates `Fluid[Water]` ports;
  - declares `MechanicalNode[Frame F, Axis DOF]` and instantiates
    `MechanicalNode[World, X]` ports;
  - records across/through variables and conservation metadata;
  - declares components and ports;
  - records domain-compatible connections;
  - keeps a `script main` entry so the file remains runnable.
- `examples/05_error_messages/port_domain_mismatch.eng`
  - verifies `E-CONNECT-DOMAIN-001`.
- `examples/05_error_messages/medium_mismatch.eng`
  - verifies `E-CONNECT-MEDIUM-001`.
- `examples/05_error_messages/frame_mismatch.eng`
  - verifies `E-CONNECT-FRAME-001`.
- `examples/05_error_messages/axis_mismatch.eng`
  - verifies `E-CONNECT-AXIS-001`.
- `examples/05_error_messages/generic_domain_arity.eng`
  - verifies `E-PORT-DOMAIN-002`.
- `examples/05_error_messages/domain_missing_across.eng`
  - verifies `E-DOMAIN-CONTRACT-001`.
- `examples/05_error_messages/domain_missing_through.eng`
  - verifies `E-DOMAIN-CONTRACT-002`.
- `examples/05_error_messages/domain_missing_conservation.eng`
  - verifies `E-DOMAIN-CONTRACT-003`.
- `examples/05_error_messages/domain_unknown_quantity.eng`
  - verifies `E-DOMAIN-VAR-001`.

## Remaining Before Metadata Preview Claim

No open items remain for the current metadata-first v2.0 preview boundary.
Domain/component syntax, structured generic metadata, domain contract
diagnostics, review/report artifacts, native IDE metadata, LSP metadata, and
official/diagnostic examples are implemented on `main`.

## Numeric Claim Boundary

Do not claim multi-domain numeric simulation until a solver/graph runtime
exists. The current v2.0 preview records conservation and connection metadata;
it does not solve the component graph.

## Deferred

- Numeric component graph solving.
- Multi-domain conservation solving.
- Domain package registry.
- Optimized native JIT/AOT execution for domain graphs.
- User-defined unit registries beyond the current quantity/unit table.

## Verification

```bat
cargo test -p eng_compiler
target\debug\eng.exe run examples\official\06_domain_port\main.eng --entry main
target\debug\eng.exe check examples\official\06_domain_port\main.eng --review
target\debug\eng.exe check examples\05_error_messages\port_domain_mismatch.eng
target\debug\eng.exe check examples\05_error_messages\medium_mismatch.eng
target\debug\eng.exe check examples\05_error_messages\frame_mismatch.eng
target\debug\eng.exe check examples\05_error_messages\axis_mismatch.eng
target\debug\eng.exe check examples\05_error_messages\generic_domain_arity.eng
target\debug\eng.exe check examples\05_error_messages\domain_missing_across.eng
target\debug\eng.exe check examples\05_error_messages\domain_missing_through.eng
target\debug\eng.exe check examples\05_error_messages\domain_missing_conservation.eng
target\debug\eng.exe check examples\05_error_messages\domain_unknown_quantity.eng
target\debug\eng.exe test examples
```

The mismatch fixtures are expected to exit non-zero with their listed
diagnostic codes.

After the runnable official fixture completes, inspect
`build\result\report_spec.json` and `build\result\report.html` for
`domain_summary`, `component_summary`, `connection_summary`,
`type_parameters`, `kind`, `name`, `display`, `package`, `version`,
`Fluid[Water]`, `MechanicalNode[World, X]`, and `domain_compatible` rows.
