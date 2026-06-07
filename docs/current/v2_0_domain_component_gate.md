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
- `across <name>: <Quantity> [unit]` records across variables.
- `through <name>: <Quantity> [unit]` records through variables.
- `conservation <text>` records the domain conservation contract as metadata.
- `component <Name> { port <name>: <Domain> }` declares component ports.
- `connect Component.port -> Other.port` records a connection.
- Port declarations referencing unknown domains produce `E-PORT-DOMAIN-001`.
- Connections between ports of different domains produce
  `E-CONNECT-DOMAIN-001`.
- Malformed connection endpoints produce `E-CONNECT-ENDPOINT-001`.
- Unresolved component ports produce `E-CONNECT-PORT-001`.
- `review.json` exposes `domain_summary`, `component_summary`, and
  `connection_summary`.
- `report_spec.json` and `report.html` expose the same domain/component surface
  so packaged runs and native IDE previews can show the declarations.
- The native IDE, LSP, and VS Code preview expose domain/component keywords and
  snippets for user testing.

## Official Fixtures

- `examples/official/06_domain_port/main.eng`
  - declares Thermal and Fluid domains;
  - records across/through variables and conservation metadata;
  - declares components and ports;
  - records domain-compatible connections;
  - keeps a `script main` entry so the file remains runnable.
- `examples/05_error_messages/port_domain_mismatch.eng`
  - verifies `E-CONNECT-DOMAIN-001`.

## Remaining Before Preview Claim

- [ ] Add native IDE domain/component inspector panels, not only completions.
- [ ] Add LSP snapshot fields or hover metadata for domain/component symbols.
- [ ] Add package/version metadata for domain declarations.
- [ ] Add at least one typed generic domain fixture such as `Fluid[Medium]`.
- [ ] Add medium/frame/axis compatibility diagnostics.
- [ ] Define whether connection summaries are ordered by source order or graph
  topology.
- [ ] Keep multi-domain numeric simulation deferred until a solver/graph runtime
  exists.

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
target\debug\eng.exe test examples
```

The mismatch fixture is expected to exit non-zero with
`E-CONNECT-DOMAIN-001`.

After the runnable official fixture completes, inspect
`build\result\report_spec.json` and `build\result\report.html` for
`domain_summary`, `component_summary`, `connection_summary`, and
`domain_compatible` rows.
