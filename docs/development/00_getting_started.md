# Getting Started

This guide explains how to start EngLang development on a Windows PC. The goal
is that every developer can clone the repository, run one setup command, and get
the same compiler/runtime/tooling environment.

## Supported Environment

```text
- Windows 10/11 x64
- Git
- Internet access for the first setup run
- PowerShell available through Windows
```

Python, Visual Studio Build Tools, and a global Rust installation are not
required for the core preview path. Setup installs the pinned Rust toolchain
inside the repository-local `.dev` folder.

## First Setup

Run from the repository root:

```bat
.\dev.bat setup
```

Setup performs:

```text
1. create .dev/cargo, .dev/rustup, and .dev/cache
2. download rustup-init.exe into .dev/cache when needed
3. install 1.78.0-x86_64-pc-windows-gnu into .dev
4. fetch locked Cargo dependencies
5. build the Rust workspace
```

All PowerShell execution goes through the common wrapper:

```bat
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\dev.ps1
```

Do not add separate PowerShell entry scripts. Add new tasks to `scripts/dev.ps1`
and expose them through `dev.bat`.

## Check Setup

```bat
.\dev.bat doctor
```

Expected shape:

```text
EngLang 1.0.0

Runtime              OK
Standard library     OK
Unit registry        OK
Plot renderer        OK
Report generator     OK
Write permission     OK
Example files        OK

Ready.
```

## Run Examples

```bat
.\dev.bat run-example
```

Or run the current official examples directly:

```bat
target\debug\eng.exe run examples\official\01_csv_plot\main.eng --entry main
target\debug\eng.exe run examples\official\02_simple_system\main.eng --entry main
```

Generated artifacts:

```text
build/
  main.engbc
  result/
    result.engres
    review.json
    report.html
    report_spec.json
    plots/
      plot_spec.json
      plot_manifest.json
      timeseries.svg
```

## Common Commands

```bat
.\dev.bat build
.\dev.bat test
.\dev.bat fmt
.\dev.bat clippy
.\dev.bat ci
.\dev.bat docs-check
.\dev.bat package
.\dev.bat package-smoke
.\dev.bat clean
```

`package-smoke` builds the portable zip, extracts it into a path containing
spaces and Korean characters, and runs the packaged `eng.exe` without relying on
Rust or Python in the target folder. It also builds a standalone bundle and runs
that bundle's `run.bat`.

## Troubleshooting

If Cargo is not found:

```bat
.\dev.bat setup
```

If generated build output looks stale:

```bat
.\dev.bat clean
.\dev.bat setup
```

If a corporate network blocks the rustup download, pre-place
`.dev/cache/rustup-init.exe` manually and run setup again. The final toolchain
installation location is still repository-local.
