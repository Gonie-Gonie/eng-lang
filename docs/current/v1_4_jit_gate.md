# v1.4 JIT Gate

This page tracks the first v1.4 JIT-start path on `main`. The current scope is
kernel discovery and lowering-plan metadata only. It does not claim native code
generation, runtime acceleration, or production JIT support.

## Current Scope

- `eng_jit` crate exists as the JIT planning boundary.
- `eng.exe jit-plan <file.eng>` emits `eng-kernel-plan-v1` JSON.
- `eng.exe jit-bench <file.eng>` emits `eng-jit-bench-v1` JSON with
  interpreter baseline measurements and `jit.status = "not_available"`.
- `--backend auto|interpreter-fallback|native-preview` records backend selection
  metadata. `native-preview` remains unavailable and selects fallback metadata.
- The native IDE Runtime Summary shows the current file's kernel plan metadata
  beside normal runtime artifacts.
- The plan uses `backend = "interpreter-fallback"` to make clear that execution
  still uses the normal runtime path.
- Hot-kernel detection currently covers:
  - TimeSeries arithmetic bindings
  - TimeSeries integration bindings
  - TimeSeries statistics fusion opportunities
  - system residuals as an interface-only RHS/Jacobian seed
- `dev.bat jit-check` validates the crate test, official CSV `jit-plan`, and
  official CSV `jit-bench --iterations 1` output.
- `dev.bat ci` runs `jit-check`.

## Completed On Main

- [x] `eng_jit` crate exposes `KernelCandidate` and `NumericKernelPlan`.
- [x] `eng-kernel-plan-v1` JSON includes format, backend, candidate count,
  candidate kind, source, reason, lowering status, and operations.
- [x] Official CSV example detects `Q_coil` TimeSeries arithmetic, `E_coil`
  integration, and `summary:Q_coil` statistics fusion candidates.
- [x] `eng test examples` checks that the official CSV example exposes v1.4
  kernel candidates.
- [x] `dev.bat jit-check` runs `cargo test -p eng_jit` and
  `eng.exe jit-plan examples\official\01_csv_plot\main.eng`.
- [x] `eng-kernel-plan-v1` compatibility rules are documented in
  [Kernel plan reference](../reference/kernel_plan.md).
- [x] Native IDE smoke verifies kernel candidate discovery, and the Runtime
  Summary panel shows format, backend, candidate count, candidate kind, source,
  reason, lowering status, and operation list.
- [x] Kernel candidates include coarse size/cost estimates: inferred row count,
  input/output count, operation-class count, scan count, complexity label, and
  notes.
- [x] `eng-jit-bench-v1` benchmark harness records interpreter timings, embeds
  the kernel plan, and explicitly marks the JIT side as not available.
- [x] Backend selection metadata is explicit for `auto`,
  `interpreter-fallback`, and `native-preview`; native preview requests are
  recorded as unavailable without executing native code.

## Remaining Before Support Claim

- [ ] Implement an actual native lowering backend only after v1.4 metadata,
  tests, and benchmark contracts remain stable across further examples.

## Verification

```bat
.\dev.bat jit-check
target\debug\eng.exe jit-plan examples\official\01_csv_plot\main.eng
target\debug\eng.exe jit-plan examples\official\01_csv_plot\main.eng --backend native-preview
target\debug\eng.exe jit-bench examples\official\01_csv_plot\main.eng --iterations 1
```
