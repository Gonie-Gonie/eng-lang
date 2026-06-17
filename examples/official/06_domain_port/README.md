# Official 06 Domain Port

Internal domain/component metadata-track user-test fixture for the open domain/port and component
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
- `ua_seed = 0.5 kW/K` inside `RoomBoundary` is recorded as
  component-local expression metadata; it is not executed as a top-level
  workflow binding.
- `connect` records component-port connections and validates domain
  compatibility.
- Compatible connections are grouped into assembly connection sets and generate
  metadata-only across/through equation seeds plus residual graph placeholders.
- The assembled graph contains three domain plans (`Thermal`, `Fluid[Water]`,
  and `MechanicalNode[World, X]`) and records `multi_domain_preview` in the
  legacy-named artifact status field.
- Connection graph review also reports duplicate edges as diagnostics and
  resolved but unconnected ports as warnings.
- A small top-level report keeps the file runnable while the domain graph
  remains a compile-time/review artifact.

Current support boundary:

- check/review metadata and connection diagnostics are supported for this
  example;
- report spec, HTML report, native IDE, and LSP snapshot metadata expose the
  domain package/version and generic argument surface;
- the native IDE Assembly panel shows component graph nodes, ports,
  connections, and source-line navigation;
- `assembly_summary` exposes component-local expression counts, generated
  connection equations, equation/unknown counts, domain plans, future
  nonlinear/DAE/delay/Predictor/adapter seed statuses, Jacobian sparsity
  placeholders, and a no-solve solver-plan placeholder;
- runtime `component_solutions` assembles generated residuals into the linear
  solver path when square, and for this underdetermined example records
  `linear_residual_satisfied_nonunique` plus the current limitation;
- numeric multi-domain simulation is not implemented;
- medium/frame/axis compatibility diagnostics are metadata checks only.
- duplicate connection and unconnected-port checks are graph-shape checks only.
- domain contract diagnostics require at least one across variable, through
  variable, and conservation line.

Useful commands:

```bat
target\debug\eng.exe check examples\official\06_domain_port\main.eng --review
target\debug\eng.exe run examples\official\06_domain_port\main.eng
```
