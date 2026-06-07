# Official 06 Domain Port

Experimental v2.0 user-test fixture for the open domain/port and component
connection surface.

This example is intentionally metadata-first:

- `domain Thermal package "eng.std.domains.thermal" version "0.1.0"` declares
  across/through variables, package metadata, version metadata, and a
  conservation contract.
- `domain Fluid[Medium M] package "eng.std.domains.fluid" version "0.1.0"`
  shows a generic domain whose component ports are instantiated as
  `Fluid[Water]`.
- `domain MechanicalNode[Frame F, Axis DOF]` shows a two-parameter domain whose
  ports are instantiated as `MechanicalNode[World, X]`.
- `component` declarations expose named `port` entries.
- `connect` records component-port connections and validates domain
  compatibility.
- `script main` keeps the file runnable while the domain graph remains a
  compile-time/review artifact.

Current support boundary:

- check/review metadata and connection diagnostics are supported for this
  example;
- report spec, HTML report, native IDE, and LSP snapshot metadata expose the
  domain package/version and generic argument surface;
- numeric multi-domain simulation is not implemented;
- medium/frame/axis compatibility diagnostics are metadata checks only.
- domain contract diagnostics require at least one across variable, through
  variable, and conservation line.

Useful commands:

```bat
target\debug\eng.exe check examples\official\06_domain_port\main.eng --review
target\debug\eng.exe run examples\official\06_domain_port\main.eng --entry main
```
