# v1.5 Standalone/AOT Gate

This page tracks the v1.5 standalone/AOT maturity path on `main`.

The current support boundary is packaged runner maturity. `eng.exe build
--standalone --profile repro` creates a reproducible bundle with `eng.exe`,
`run.bat`, source, bytecode, `.engpkg`, `.lock`, Args help, review HTML, and
bundled CSV dependencies. It is not an optimized native `model.exe` AOT
compiler yet.

## Current Scope

- `eng.exe build <file.eng> --entry <name> --standalone --profile repro`
  creates `dist/<model>-standalone`.
- The bundle includes the runtime executable as `eng.exe`.
- `run.bat --help` prints Args-derived help.
- `run.bat --<field> <value>` forwards flags to `eng.exe run`, and generated
  artifacts record Args values.
- `.engpkg` records package format, runtime ABI, profile, source root, artifact
  root, source/bytecode paths and hashes, Args metadata, dependency count,
  dependency paths, and dependency hashes.
- `.lock` records runtime/compiler/package/bytecode/result/report/plot format
  versions, repro profile, source/bytecode hashes, entry name, dependency count,
  and dependency hashes.

## Completed On Main

- [x] Packaged runner bundle works in repo and portable package smoke paths.
- [x] Runtime executable bundling uses the current `eng.exe`.
- [x] Args-derived standalone help is generated as `ARGS_HELP.txt`.
- [x] CSV dependencies are copied under `source/` and cannot escape the bundle.
- [x] `.engpkg` exposes dependency paths and hashes.
- [x] `.lock` exposes package/runtime ABI and dependency hash data.
- [x] `artifacts-check` validates package manifest and lock fields.
- [x] `package-smoke` creates a clean extracted portable folder, runs the
  standalone bundle with a non-default `--input` override, and checks
  `result.engres.arg_values`.

## Remaining Before Support Claim

- [ ] Add a user-facing standalone package reference page with a field table.
- [ ] Add native `model.exe` or document a reserved executable-wrapper plan.
- [ ] Add full binary dependency hashing before non-text assets are bundled.
- [ ] Keep optimized AOT/native compilation explicitly deferred until a real
  backend exists.

## Verification

```bat
.\dev.bat artifacts-check
.\dev.bat package-smoke
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat --help
dist\main-standalone\run.bat --input data/sensor.csv
```
