# Official 06 Domain Port

Experimental domain/component track user-test fixture for the open domain/port and component
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
- Compatible connections are grouped into assembly connection sets and generate
  metadata-only across/through equation seeds plus residual graph placeholders.
- Connection graph review also reports duplicate edges as diagnostics and
  resolved but unconnected ports as warnings.
- A small top-level report keeps the file runnable while the domain graph
  remains a compile-time/review artifact.

Current support boundary:

- check/review metadata and connection diagnostics are supported for this
  example;
- report spec, HTML report, native IDE, and LSP snapshot metadata expose the
  domain package/version and generic argument surface;
- `assembly_summary` exposes generated connection equations, equation/unknown
  counts, Jacobian sparsity placeholders, and a no-solve solver-plan placeholder;
- runtime `component_solutions` evaluates the homogeneous connection
  constraints, reports `fixed_point_converged`, and records the current
  non-unique/underdetermined limitation;
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
