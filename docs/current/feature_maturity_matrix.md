# Feature Maturity Matrix

Use this matrix to avoid treating an official example seed as a general
language feature. Status terms are defined at the bottom of this file.

| Feature | Status | Current Scope | Main Limitation | Next Action |
|---|---|---|---|---|
| Fast `=` declaration | Supported | Local/script expressions and official examples | Broader language contexts still need care | Maintain and document diagnostics |
| `:=` rejection | Supported | Parser/compiler diagnostic | None for current public syntax | Maintain |
| Dimensionless policy | Supported | Addition/subtraction and expected-type diagnostics | Broader algebra coverage grows with expression support | Maintain and expand tests |
| Quantity/unit registry | Supported | Built-in quantities/units and IDE completions | User-defined units deferred | Maintain |
| `degC` / `°C` temperature spelling | Supported | `degC` canonical ASCII spelling; `°C` user-facing alias for AbsoluteTemperature | Broader Unicode unit aliases deferred | Maintain alias tests and docs |
| CSV promote | Supported | Official typed schema import path | Arbitrary table formula support deferred | Generalize table expressions later |
| DateTime index | Supported | Official CSV TimeSeries path | More calendar/timezone semantics deferred | Maintain metadata |
| Missing policies | Preview | Official/data-quality paths | General policy DSL is limited | Harden policy semantics |
| Constraints | Preview | Data-quality examples and review metadata | General constraint runtime is limited | Expand supported checks |
| TimeSeries statistics | Supported | Official HeatRate TimeSeries path | General quantity rules and arbitrary TimeSeries expressions limited | Expand quantity-aware kernels |
| `integrate(... over Time)` | Supported | HeatRate to Energy metadata and supported example | Wider signal types deferred | Generalize integration rules |
| PlotSpec line plot | Supported | CSV-derived TimeSeries line plot and SVG | Multi-series and interactive consistency deferred | Expand PlotSpec semantics |
| Bar/histogram plot seeds | Preview | Report/PlotSpec seed tests, uncertainty histogram bins, and IDE preview rendering | General histogram expressions and grouped bar semantics deferred | Harden before support claim |
| Report/review artifacts | Supported | Official artifacts, schemas, report spec, review JSON | Rich report layout remains limited | Maintain schemas and improve IDE panels |
| Minimal `system`/`eq` | Preview | One-state thermal system and unit diagnostics | Multi-state/nonlinear/adaptive solving deferred | Clarify solver boundary |
| Args binding | Supported | String/path `--input` binding and help metadata | Bool/count/unit/duration conversion deferred | Add typed Args conversion |
| Standalone package | Supported | Official package and package-smoke path with Args help, runtime bundling, dependency copy/byte-hash metadata, `.engpkg`, `.lock`, and package reference | Optimized native model.exe/AOT deferred | Maintain v1.5 package contract |
| Example taxonomy | Supported | `examples/official` user-test namespace, compatibility regression examples, diagnostic fixtures, and data-quality fixtures | Historical release notes may mention older paths | Maintain IDE/CLI ordering and package docs |
| Native tester IDE | Preview | Open/check/save/run, diagnostics, completions, variable/unit/schema/CSV inspector, PlotSpec preview, runtime summary | Not a full LSP/editor platform | v1.0.3 hardening |
| VS Code extension | Preview | Packaged diagnostic/completion shape with optional `eng-lsp --snapshot` backend | Secondary editor path, not a persistent LSP client yet | Keep package smoke stable |
| Integrated HVAC example | Preview | User-test workflow across supported subsystems | It is not proof of general solver/table support | Use as release manual test |
| Uncertainty core | Experimental | Official example, deterministic samples, source diagnostics, propagation transform/source metadata, histogram artifact path on `main` | v1.1 release gate not completed | v1.1 |
| Data-driven modeling / ML | Experimental | Official parity/residual examples, artifacts, source validation diagnostics, argument diagnostics, and v1.2 gate on `main` | Not release-supported | v1.2 gate |
| LSP | Experimental | `eng-lsp.exe` smoke/snapshot, stdio round-trip tests, package-smoke inclusion, optional VS Code snapshot backend, line diagnostics, context-aware schema column completion, hover, and minimal stdio JSON-RPC on `main` | Not editor-validated as a release path | v1.3 gate |
| JIT kernel planning | Experimental | `eng_jit`, `eng.exe jit-plan`, `eng.exe jit-bench`, backend selection metadata, native IDE Runtime Summary display, and `eng-kernel-plan-v1`/`eng-jit-bench-v1` metadata with coarse size/cost estimates for TimeSeries arithmetic/statistics/integration and system residual interface seeds | No native code generation or speedup claim | v1.4 gate |
| Domain/component platform | Experimental | `domain`, `across`, `through`, `conservation`, `component`, `port`, and `connect` metadata with official fixture and invalid domain connection diagnostics | No numeric multi-domain solver, generic medium/frame/axis system, or package registry | v2.0 gate |

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
