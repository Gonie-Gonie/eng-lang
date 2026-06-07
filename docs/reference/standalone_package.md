# Standalone Package Reference

This page defines the supported v1.5 packaged-runner contract for
`eng.exe build <file.eng> --entry <name> --standalone --profile repro`.

The supported artifact is a reproducible Windows package directory that runs
through the bundled EngLang runtime. It is not an optimized native AOT
executable. Native `model.exe` generation is reserved for a later backend gate.

## Support Boundary

| Area | Supported in v1.5 | Not Claimed |
|---|---|---|
| Runner | `run.bat` invokes bundled `eng.exe run` | Optimized native execution |
| Runtime | Current `eng.exe` is copied into the bundle | Separate single-model runtime binary |
| Source | Entry source is copied under `source/` | Arbitrary project tree packaging |
| Dependencies | Relative CSV promotions are copied under `source/` | Registry packages or binary asset bundles |
| Args | `struct Args` help and forwarded CLI flags | Typed conversion beyond current string/path values |
| Reproducibility | Manifest, lock, hashes, profile metadata | Cryptographic supply-chain attestation |

## Bundle Layout

`eng build --standalone --profile repro` writes:

```text
dist/
  <model>-standalone/
    eng.exe
    run.bat
    ARGS_HELP.txt
    <model>.engpkg
    <model>.lock
    <model>.engbc
    <model>.review.html
    source/
      <file.eng>
      data/
        sensor.csv
```

| Path | Purpose |
|---|---|
| `eng.exe` | Bundled runtime CLI used by `run.bat`. |
| `run.bat` | Execution wrapper for target PCs. It accepts `--help` and forwards Args flags. |
| `ARGS_HELP.txt` | Args-derived help generated from the selected entry signature. |
| `<model>.engpkg` | Human-readable package manifest for the packaged runner. |
| `<model>.lock` | Reproducibility lock with runtime/compiler/artifact format versions. |
| `<model>.engbc` | Bytecode v1 generated for the selected entry. |
| `<model>.review.html` | Build-time review page for the packaged source. |
| `source/` | Packaged source root and bundled data dependencies. |
| `build/result/` | Created by `run.bat`; contains normal run outputs. |

## Running The Package

```bat
dist\main-standalone\run.bat --help
dist\main-standalone\run.bat
dist\main-standalone\run.bat --input data/sensor.csv
```

`run.bat --help` prints `ARGS_HELP.txt`. Any extra `--<field> <value>` flags
are forwarded to `eng.exe run source\<file.eng> --entry <name>` and are recorded
in `build/result/result.engres` under `arg_values`.

Relative Args paths are interpreted from the packaged `source/` directory for
the current official CSV workflow. For example, `--input data/sensor.csv`
resolves to `source/data/sensor.csv` inside the bundle.

## `.engpkg` Fields

The package manifest is a stable key/value text artifact. A normalized view is
validated by `docs/schemas/engpkg.schema.json`.

| Field | Example | Meaning |
|---|---|---|
| `format` | `engpkg-stable-1` | Manifest contract identifier. |
| `package_format_version` | `1` | Numeric package format version. |
| `runtime_abi` | `eng-runtime-cli-v1` | Runtime CLI ABI expected by `run.bat`. |
| `profile` | `repro` | Reproducible package profile. |
| `runner` | `run.bat` | User-facing package launcher. |
| `engine` | `eng.exe` | Runtime executable bundled beside `run.bat`. |
| `source_root` | `source` | Root containing packaged source and data dependencies. |
| `artifact_root` | `build/result` | Runtime output directory created by the runner. |
| `source` | `source/main.eng` | Packaged entry source path. |
| `bytecode` | `main.engbc` | Packaged bytecode path. |
| `source_hash` | `<hash>` | Stable fingerprint of packaged source text. |
| `bytecode_hash` | `<hash>` | Stable fingerprint of bytecode text. |
| `entry_name` | `main` | Selected entry name. |
| `entry` | `script main(args: Args) -> Report` | Selected entry signature. |
| `args_schema` | `Args` | Args struct name, or `-` when absent. |
| `args_field_count` | `1` | Number of Args fields found for help/binding. |
| `args_help` | `ARGS_HELP.txt` | Args help artifact path. |
| `dependency_count` | `1` | Number of bundled data dependencies. |
| `dependencies` | `source/data/sensor.csv` | Semicolon-separated dependency paths, or `-`. |
| `dependency_hashes` | `source/data/sensor.csv:<hash>` | Semicolon-separated dependency path/hash pairs, or `-`. |

## `.lock` Fields

The lock records the runtime and artifact versions needed to reproduce the
packaged-runner behavior.

| Field | Example | Meaning |
|---|---|---|
| `runtime_version` | `1.0.3` | Runtime version embedded in the bundled CLI. |
| `compiler_version` | `1.0.3` | Compiler/build version used to create the package. |
| `package_format_version` | `1` | Package format used by `.engpkg`. |
| `runtime_abi` | `eng-runtime-cli-v1` | CLI ABI expected by the runner. |
| `bytecode_version` | `1` | Bytecode format version. |
| `result_format_version` | `1` | `result.engres` format version. |
| `report_schema_version` | `1` | ReportSpec schema version. |
| `plot_spec_version` | `1` | PlotSpec schema version. |
| `profile` | `repro` | Reproducible package profile. |
| `source_hash` | `<hash>` | Source fingerprint at package build time. |
| `bytecode_hash` | `<hash>` | Bytecode fingerprint at package build time. |
| `entry_name` | `main` | Selected entry name. |
| `dependency_count` | `1` | Number of bundled data dependencies. |
| `dependency_hashes` | `source/data/sensor.csv:<hash>` | Dependency fingerprints at package build time. |

## Hash Semantics

Package hashes are stable content fingerprints used by artifact checks and
clean-folder smoke tests. They are 16-character lowercase hexadecimal FNV-1a
fingerprints. They are not security checksums.

Dependency hashes are computed over raw file bytes, not UTF-8 text. This keeps
the contract ready for future non-text assets even though the current supported
dependency bundling path is relative CSV promotion.

Dependency paths are normalized with `/` separators and are relative to the
bundle root. Multiple entries are sorted lexicographically and separated with
semicolons.

## Reserved `model.exe` Plan

The v1.5 package reserves a future executable-wrapper/AOT boundary without
claiming it now:

1. `run.bat` remains the supported v1.5 launcher.
2. `engine = eng.exe` means the packaged runner depends on the general EngLang
   CLI runtime.
3. A future `model.exe` must either embed a selected entry and runtime ABI or
   act as a thin executable wrapper with equivalent `.engpkg` and `.lock`
   metadata.
4. A future optimized AOT backend must update this reference, the package
   schema, `release-check`, package smoke, and user documentation before it can
   be claimed.

## Verification

```bat
.\dev.bat artifacts-check
.\dev.bat package-smoke
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat --help
dist\main-standalone\run.bat --input data/sensor.csv
```
