# Official 06 Domain Port

Experimental v2.0 user-test fixture for the open domain/port and component
connection surface.

This example is intentionally metadata-first:

- `domain Thermal` declares across/through variables and a conservation
  contract.
- `domain Fluid` shows a second user-defined domain for compatibility checks.
- `component` declarations expose named `port` entries.
- `connect` records component-port connections and validates domain
  compatibility.
- `script main` keeps the file runnable while the domain graph remains a
  compile-time/review artifact.

Current support boundary:

- check/review metadata and connection diagnostics are supported for this
  example;
- numeric multi-domain simulation is not implemented;
- medium/frame/axis generic domains and package versioning remain later v2.0
  milestones.

Useful commands:

```bat
target\debug\eng.exe check examples\official\06_domain_port\main.eng --review
target\debug\eng.exe run examples\official\06_domain_port\main.eng --entry main
```
