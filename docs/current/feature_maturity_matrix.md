# Feature Maturity Matrix

Use this matrix to avoid treating an official example seed as a general
language feature. Stable-core support is narrower than implementation on
`main`.

| Feature | Status | Current Scope | Main Limitation | Next Action |
|---|---|---|---|---|
| Fast `=` declaration | Stable | Local/top-level workflow bindings and official examples | Broader language contexts still need care | Maintain diagnostics |
| `:=` rejection | Stable | Parser/compiler diagnostic | None for current public syntax | Maintain |
| Dimensionless policy | Stable | Addition/subtraction and expected-type diagnostics | Broader algebra coverage grows with expression support | Expand tests |
| Quantity/unit registry | Stable | Built-in quantities/units and IDE completions | User-defined units deferred | Maintain |
| `degC` / `°C` temperature spelling | Stable | `degC` canonical ASCII spelling; `°C` alias for AbsoluteTemperature | Broader Unicode unit aliases deferred | Maintain alias tests and docs |
| Top-level execution, `args`, `const`, functions, and file imports | Stable | Files without `script` run as top-level workflows; `args { ... }` supports primitive/path defaults and CLI overrides; top-level `const` is importable; `fn` supports typed parameters, function locals, checked return expressions, relative file imports, and print/export runtime evaluation for scalar calls | Package/module imports, multi-return functions, and broad expression evaluation deferred | Harden IDE visibility and formatter support |
| Command-style verbs, `where`, and `with` | Preview | Parenthesis-light syntax for built-in workflow verbs only; canonical lowering for integrate/mean/max/min/plot-style commands; owner-local `where` bindings; `with` option/display blocks; review metadata and policy diagnostics | Arbitrary user-defined function command syntax, broad command runtime semantics, and project-wide display unit policy deferred | Harden examples, formatter, IDE display, and option schema |
| CSV promote | Stable | Official typed schema import path | Arbitrary table formula support deferred | Generalize table expressions later |
| DateTime index | Stable | Official CSV TimeSeries path | More calendar/timezone semantics deferred | Maintain metadata |
| Missing policies | Preview | Official/data-quality paths | General policy DSL is limited | Harden policy semantics |
| Constraints | Preview | Data-quality examples and review metadata | General constraint runtime is limited | Expand supported checks |
| TimeSeries statistics | Stable | Official HeatRate TimeSeries path plus `timeseries_kernels` metadata for the table heat-rate kernel | General quantity rules and arbitrary TimeSeries expressions limited | Expand quantity-aware kernels |
| `integrate(... over Time)` | Stable | HeatRate to Energy metadata and supported example | Wider signal types deferred | Generalize integration rules |
| Unit-aware `print` and summary CSV export | Stable | Type-checked print interpolation, scalar statistics, integration values, and explicit one-row `export summary to csv` files under `build/result` | First-class Summary object deferred by decision record; table/TimeSeries CSV export deferred; display policy is out of scope | Maintain examples and artifact metadata |
| PlotSpec line plot | Stable | CSV-derived TimeSeries line plot, measured-vs-simulated multi-series line plot, and SVG | Interactive consistency, grouped plots, and broader axis semantics deferred | Expand PlotSpec semantics |
| Bar/histogram plot paths | Preview | Report/PlotSpec tests, raw-value histogram bins, uncertainty histogram bins, ML residual bars, and IDE rendering | Multiple histogram series, custom bin counts, and grouped/stacked bar semantics deferred | Harden before support claim |
| Report/review artifacts | Stable | Official artifacts, schemas, report spec, review JSON | Rich report layout remains limited | Maintain schemas and improve IDE panels |
| Minimal `system`/`eq` | Preview | One-state thermal system, unit diagnostics, preview ODE runner output, fixed-step solver metadata, and `sim.T_zone` TimeSeries for the measured-vs-simulated example | Multi-state/nonlinear/adaptive solving deferred | Clarify solver boundary |
| Args binding | Stable | `args { ... }` only, String/path/CsvFile/DirectoryPath, Bool, Int/Count, Float/Number, Duration normalization, dynamic pure defaults, CLI overrides, and help metadata | Quantity/unit-literal Args and flag-only booleans deferred | Maintain typed conversion and side-effect diagnostics |
| Measured-vs-simulated workflow | Stable | Official workflow promotes measured/weather CSV data, simulates a one-state thermal system into typed TimeSeries, computes `rmse measured vs sim`, validates thresholds, emits time-alignment metadata, and plots measured plus simulated series together | Calibration, multi-state simulation, resampling policy controls, and full solver selection deferred | Maintain official example and artifact schemas |
| Side-effect/general programming policy | Stable | v1.0.0 includes typed path defaults/helpers, provenance-visible `exists`, read-only UTF-8 `read text/json/toml` expressions with source hashes, explicit `write text/json`, idempotent overwrite hardening for write/export outputs, `output_manifest.json`, constrained copy/move/delete file operation metadata, `log debug/info/warn/error` messages with `run_log.json`, explicit `run command` process execution with `ProcessResult` and `process_results.json`, named `test` blocks with `assert`/`golden` checks plus `test_results.json`, and `eng run --profile safe|normal|repro` basics | Structured JSON/TOML values, broad filesystem mutation outside generated-output boundaries, network, workspace-wide test discovery, and full process sandboxing are not stable-supported | Maintain stable boundaries |
| Standalone package | Stable | Official package and package-smoke path with Args help, runtime bundling, dependency copy/byte-hash metadata, `.engpkg`, `.lock`, and package reference | Optimized native model.exe/AOT deferred | Maintain package contract |
| Example taxonomy | Stable | `examples/official` user-test namespace, compatibility regression examples, diagnostic fixtures, and data-quality fixtures | Historical naming exists only in git history | Maintain IDE/CLI ordering and package docs |
| Tauri tester IDE | Preview | Static Tauri/WebView app with collapsible explorer, open editors, multi-file tabs, check/save/run, Problems/Terminal bottom panel, caret completions, Variables/Plot/Run inspector tabs, run-directory prompt, and PlotSpec preview | Not a full LSP/editor platform; rich domain/schema inspectors remain future hardening | Continue IDE/LSP track hardening |
| VS Code extension | Preview | Packaged diagnostic/completion shape with optional `eng-lsp --snapshot` backend | Secondary editor path, not a persistent LSP client yet | Keep package smoke stable |
| Integrated HVAC example | Preview | User-test workflow across supported subsystems | It is not proof of general solver/table support | Use as manual preview test |
| Uncertainty track | Experimental | Official example, deterministic samples, source and argument diagnostics, propagation transform/source metadata, histogram artifact path | Not stable-supported; full Monte Carlo/Jacobian propagation deferred | Track work in `docs/current/tracks.md` |
| Data-driven modeling track | Experimental | Official parity/residual examples, artifacts, source validation diagnostics, argument diagnostics | Not stable-supported | Track work in `docs/current/tracks.md` |
| LSP track | Experimental | `eng-lsp.exe` smoke/snapshot, stdio round-trip tests, package-smoke inclusion, optional VS Code snapshot backend, diagnostics, completion, hover | Not editor-validated as a stable release path | Track work in `docs/current/tracks.md` |
| Runtime optimization/JIT/AOT track | Experimental | `eng_jit`, `eng.exe jit-plan`, `eng.exe jit-bench`, backend selection metadata, IDE kernel-plan display, and metadata estimates | No native code generation or speedup claim | Track work in `docs/current/tracks.md` |
| Domain/component track | Experimental | Domain/component/port/connect metadata, diagnostics, metadata-only assembly connection sets, generated connection equations, residual graph placeholders, domain-plan based multi-domain preview metadata, homogeneous connection-constraint solver preview artifacts, review/report sections, IDE inspector, LSP hover/completion metadata | No production numeric multi-domain solver, boundary-condition solve, component behavior solve, or package registry | Track work in `docs/current/tracks.md` |
| Class/domain-object track | Supported preview | `class` declarations, typed fields/defaults, object literals, nested object references, field access metadata, simple class validation blocks, zero-argument metadata methods, immutable copy-with metadata, diagnostics, `class_summary`/`object_summary` artifacts, IDE/LSP metadata, and official `19_class_object_preview` | Runtime object dispatch/lowering, method arguments, mutation, and inheritance deferred | Keep classes reviewable and avoid presenting them as replacements for systems/components |

## Stable-Core Solver Vocabulary

Use these labels consistently in public docs:

| Term | Current Meaning |
|---|---|
| Preview ODE runner | The narrow runtime path used by official one-state thermal examples. |
| One-state fixed-step thermal preview | The supported preview shape for `02_simple_system` and measured-vs-simulated simulation output. |
| Solver metadata | Review/result/report-spec metadata describing recognized equations, boundaries, methods, and limitations. |
| Solver plan | Planning metadata for equation ordering and future numeric work; not proof of a general solver. |
| General solver | Future only. Do not present as current supported behavior. |
| Component graph solver | Future only. Domain/component metadata may exist without numeric multi-domain solving. |

## Status Terms

| Status | Meaning |
|---|---|
| Prototype | Internal spike or seed. Do not present as a release feature. |
| Preview | Usable through official examples or package paths with explicit limitations. |
| Supported preview | Documented, tested, and visible through diagnostics or IDE metadata for the stated narrow scope, but not yet covered by stable breaking-change policy. |
| Stable | Public behavior covered by the 1.0.0 stable-core scope and breaking-change policy. |
| Experimental | May exist on `main`, but is not stable-supported. |
| Planned | Intended future work with no supported implementation contract yet. |

## Completion Policy

A feature is not complete merely because an example passes. A feature is
complete only when its language rule, parser/AST support, semantic/type/unit
check, runtime/check behavior, diagnostics, IDE metadata or inspector support,
report/review artifact, official example, tests, and documentation are aligned
for the stated scope.
