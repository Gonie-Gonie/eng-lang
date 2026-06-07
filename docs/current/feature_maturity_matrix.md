# Feature Maturity Matrix

Use this matrix to avoid treating an official example seed as a general
language feature. Status terms are defined at the bottom of this file.

| Feature | Status | Current Scope | Main Limitation | Next Action |
|---|---|---|---|---|
| Fast `=` declaration | Supported | Local/script expressions and official examples | Broader language contexts still need care | Maintain and document diagnostics |
| `:=` rejection | Supported | Parser/compiler diagnostic | None for current public syntax | Maintain |
| Dimensionless policy | Supported | Addition/subtraction and expected-type diagnostics | Broader algebra coverage grows with expression support | Maintain and expand tests |
| Quantity/unit registry | Supported | Built-in quantities/units and IDE completions | User-defined units deferred | Maintain |
| `degC` temperature spelling | Supported | Canonical docs/examples spelling | User-facing `°C` alias is not clarified | Decide alias policy |
| CSV promote | Supported | Official typed schema import path | Arbitrary table formula support deferred | Generalize table expressions later |
| DateTime index | Supported | Official CSV TimeSeries path | More calendar/timezone semantics deferred | Maintain metadata |
| Missing policies | Preview | Official/data-quality paths | General policy DSL is limited | Harden policy semantics |
| Constraints | Preview | Data-quality examples and review metadata | General constraint runtime is limited | Expand supported checks |
| TimeSeries statistics | Supported | Official HeatRate TimeSeries path | General quantity rules and arbitrary TimeSeries expressions limited | Expand quantity-aware kernels |
| `integrate(... over Time)` | Supported | HeatRate to Energy metadata and supported example | Wider signal types deferred | Generalize integration rules |
| PlotSpec line plot | Supported | CSV-derived TimeSeries line plot and SVG | Multi-series and interactive consistency deferred | Expand PlotSpec semantics |
| Bar/histogram plot seeds | Preview | Report/PlotSpec seed tests and IDE preview rendering | True histogram binning and full semantics deferred | Harden before support claim |
| Report/review artifacts | Supported | Official artifacts, schemas, report spec, review JSON | Rich report layout remains limited | Maintain schemas and improve IDE panels |
| Minimal `system`/`eq` | Preview | One-state thermal system and unit diagnostics | Multi-state/nonlinear/adaptive solving deferred | Clarify solver boundary |
| Args binding | Supported | String/path `--input` binding and help metadata | Bool/count/unit/duration conversion deferred | Add typed Args conversion |
| Standalone package | Supported | Official package and package-smoke path | Broader platform matrix deferred | Maintain Windows portable path |
| Native tester IDE | Preview | Open/check/save/run, diagnostics, completions, PlotSpec preview, runtime summary | Not a full LSP/editor platform | v1.0.3 hardening |
| VS Code extension | Preview | Packaged diagnostic/completion shape | Secondary editor path | Keep package smoke stable |
| Integrated HVAC example | Preview | User-test workflow across supported subsystems | It is not proof of general solver/table support | Use as release manual test |
| Uncertainty core | Experimental | Official seed example and artifacts on `main` | Not release-supported | v1.1 |
| Data-driven modeling / ML | Experimental | Official seed example and artifacts on `main` | Not release-supported | v1.2 |
| LSP | Planned | Not an official runtime path | Native IDE currently owns tester workflow | v1.3 or later |

## Status Terms

| Status | Meaning |
|---|---|
| Prototype | Internal spike or seed. Do not present as a release feature. |
| Preview | Usable through official examples or package paths with explicit limitations. |
| Supported | Documented, tested, has diagnostics or IDE metadata where relevant, and is part of the release-target contract. |
| Stable | Public behavior with a breaking-change policy. |
| Experimental | May exist on `main`, but is not release-supported. |
| Planned | Intended future work with no supported implementation contract yet. |

## Completion Policy

A feature is not complete merely because an example passes. A feature is
complete only when its language rule, compiler check, runtime or check
behavior, diagnostic, IDE metadata, official example, and documentation are
aligned for the stated scope.
