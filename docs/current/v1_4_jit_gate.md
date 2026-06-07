# v1.4 JIT Gate

This page tracks the first v1.4 JIT-start path on `main`. The current scope is
kernel discovery and lowering-plan metadata only. It does not claim native code
generation, runtime acceleration, or production JIT support.

## Current Scope

- `eng_jit` crate exists as the JIT planning boundary.
- `eng.exe jit-plan <file.eng>` emits `eng-kernel-plan-v1` JSON.
- The plan uses `backend = "interpreter-fallback"` to make clear that execution
  still uses the normal runtime path.
- Hot-kernel detection currently covers:
  - TimeSeries arithmetic bindings
  - TimeSeries integration bindings
  - TimeSeries statistics fusion opportunities
  - system residuals as an interface-only RHS/Jacobian seed
- `dev.bat jit-check` validates the crate test and official CSV `jit-plan`
  output.
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

## Remaining Before Support Claim

- [ ] Feed JIT plan summaries into the native IDE Runtime/Inspector panel.
- [ ] Add candidate cost/size estimates rather than simple heuristic reasons.
- [ ] Add a benchmark harness that compares interpreter and future JIT paths
  without making speedup claims.
- [ ] Add native lowering backend selection only after the metadata contract and
  tests are stable.

## Verification

```bat
.\dev.bat jit-check
target\debug\eng.exe jit-plan examples\official\01_csv_plot\main.eng
```
