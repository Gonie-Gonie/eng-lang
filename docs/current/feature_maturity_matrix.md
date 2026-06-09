# Feature Maturity Matrix

Use this matrix to avoid treating an official example seed as a general
language feature. Public preview support is narrower than implementation on
`main`.

| Feature | Status | Current Scope | Main Limitation | Next Action |
|---|---|---|---|---|
| Fast `=` declaration | Supported preview | Local/script expressions and official examples | Broader language contexts still need care | Maintain diagnostics |
| `:=` rejection | Supported preview | Parser/compiler diagnostic | None for current public syntax | Maintain |
| Dimensionless policy | Supported preview | Addition/subtraction and expected-type diagnostics | Broader algebra coverage grows with expression support | Expand tests |
| Quantity/unit registry | Supported preview | Built-in quantities/units and IDE completions | User-defined units deferred | Maintain |
| `degC` / `°C` temperature spelling | Supported preview | `degC` canonical ASCII spelling; `°C` alias for AbsoluteTemperature | Broader Unicode unit aliases deferred | Maintain alias tests and docs |
| Top-level execution, `args`, `const`, functions, and file imports | Preview | Files without `script` run as top-level workflows; `args { ... }` supports primitive/path defaults and CLI overrides; top-level `const` is importable; `fn` supports typed parameters, function locals, checked return expressions, relative file imports, and print/export runtime evaluation for scalar calls | Package/module imports, multi-return functions, broad expression evaluation, and general side-effect/provenance profiles deferred | Harden IDE visibility and formatter support |
| Command-style verbs, `where`, and `with` | Preview | Parenthesis-light syntax for built-in workflow verbs only; canonical lowering for integrate/mean/max/min/plot-style commands; owner-local `where` bindings; `with` option/display blocks; review metadata and policy diagnostics | Arbitrary user-defined function command syntax, broad command runtime semantics, and project-wide display unit policy deferred | Harden examples, formatter, IDE display, and option schema |
| CSV promote | Supported preview | Official typed schema import path | Arbitrary table formula support deferred | Generalize table expressions later |
| DateTime index | Supported preview | Official CSV TimeSeries path | More calendar/timezone semantics deferred | Maintain metadata |
| Missing policies | Preview | Official/data-quality paths | General policy DSL is limited | Harden policy semantics |
| Constraints | Preview | Data-quality examples and review metadata | General constraint runtime is limited | Expand supported checks |
| TimeSeries statistics | Supported preview | Official HeatRate TimeSeries path plus `timeseries_kernels` metadata for the preview table heat-rate kernel | General quantity rules and arbitrary TimeSeries expressions limited | Expand quantity-aware kernels |
| `integrate(... over Time)` | Supported preview | HeatRate to Energy metadata and supported example | Wider signal types deferred | Generalize integration rules |
| Unit-aware `print` and summary CSV export | Preview | Type-checked print interpolation, scalar statistics, integration values, and explicit one-row `export summary to csv` files under `build/result` | First-class Summary object deferred by decision record; table/TimeSeries CSV export deferred; display policy is out of scope | Harden examples and artifact metadata |
| PlotSpec line plot | Supported preview | CSV-derived TimeSeries line plot and SVG | Multi-series and interactive consistency deferred | Expand PlotSpec semantics |
| Bar/histogram plot paths | Preview | Report/PlotSpec tests, raw-value histogram bins, uncertainty histogram bins, ML residual bars, and IDE rendering | Multiple histogram series, custom bin counts, and grouped/stacked bar semantics deferred | Harden before support claim |
| Report/review artifacts | Supported preview | Official artifacts, schemas, report spec, review JSON | Rich report layout remains limited | Maintain schemas and improve IDE panels |
| Minimal `system`/`eq` | Preview | One-state thermal system and unit diagnostics | Multi-state/nonlinear/adaptive solving deferred | Clarify solver boundary |
| Args binding | Supported preview | `args { ... }` only, String/path/CsvFile/DirectoryPath, Bool, Int/Count, Float/Number, Duration normalization, dynamic pure defaults, CLI overrides, and help metadata | Quantity/unit-literal Args and flag-only booleans deferred | Maintain typed conversion and side-effect diagnostics |
| Side-effect/general programming policy | Preview | v0.6 implements typed path defaults/helpers, provenance-visible `exists`, read-only UTF-8 `read text/json/toml` expressions with source hashes, explicit `write text/json`, idempotent overwrite hardening for write/export outputs, `output_manifest.json`, and constrained copy/move/delete file operation metadata | Structured JSON/TOML values, broad filesystem mutation outside generated-output boundaries, process execution, and network are not public-supported | Add log/warn/run-log artifact seed |
| Standalone package | Supported preview | Official package and package-smoke path with Args help, runtime bundling, dependency copy/byte-hash metadata, `.engpkg`, `.lock`, and package reference | Optimized native model.exe/AOT deferred | Maintain package contract |
| Example taxonomy | Supported preview | `examples/official` user-test namespace, compatibility regression examples, diagnostic fixtures, and data-quality fixtures | Historical naming exists only in git history | Maintain IDE/CLI ordering and package docs |
| Native tester IDE | Preview | Open/check/save/run, diagnostics, completions, variable/unit/schema/CSV/domain graph inspector, PlotSpec preview, runtime summary, UI settings | Not a full LSP/editor platform | Continue IDE/LSP track hardening |
| VS Code extension | Preview | Packaged diagnostic/completion shape with optional `eng-lsp --snapshot` backend | Secondary editor path, not a persistent LSP client yet | Keep package smoke stable |
| Integrated HVAC example | Preview | User-test workflow across supported subsystems | It is not proof of general solver/table support | Use as manual preview test |
| Uncertainty track | Experimental | Official example, deterministic samples, source and argument diagnostics, propagation transform/source metadata, histogram artifact path | Not public-supported; full Monte Carlo/Jacobian propagation deferred | Track work in `docs/current/tracks.md` |
| Data-driven modeling track | Experimental | Official parity/residual examples, artifacts, source validation diagnostics, argument diagnostics | Not public-supported | Track work in `docs/current/tracks.md` |
| LSP track | Experimental | `eng-lsp.exe` smoke/snapshot, stdio round-trip tests, package-smoke inclusion, optional VS Code snapshot backend, diagnostics, completion, hover | Not editor-validated as a release path | Track work in `docs/current/tracks.md` |
| Runtime optimization/JIT/AOT track | Experimental | `eng_jit`, `eng.exe jit-plan`, `eng.exe jit-bench`, backend selection metadata, IDE kernel-plan display, and metadata estimates | No native code generation or speedup claim | Track work in `docs/current/tracks.md` |
| Domain/component track | Experimental | Domain/component/port/connect metadata, diagnostics, review/report sections, IDE inspector, LSP hover/completion metadata | No numeric multi-domain solver or package registry | Track work in `docs/current/tracks.md` |
| Class/domain-object track | Planned | v0.2 documents class as a reviewable typed engineering object concept | No class declaration/object literal/runtime lowering public support | Keep as a track until minimal object semantics exist |

## Status Terms

| Status | Meaning |
|---|---|
| Prototype | Internal spike or seed. Do not present as a release feature. |
| Preview | Usable through official examples or package paths with explicit limitations. |
| Supported preview | Documented, tested, has diagnostics or IDE metadata where relevant, and is part of the current public preview contract. |
| Stable | Public behavior with a breaking-change policy. |
| Experimental | May exist on `main`, but is not public-supported. |
| Planned | Intended future work with no supported implementation contract yet. |

## Completion Policy

A feature is not complete merely because an example passes. A feature is
complete only when its language rule, compiler check, runtime/check behavior,
diagnostic, IDE metadata, official example, and documentation are aligned for
the stated scope.
