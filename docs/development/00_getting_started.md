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

Visual Studio Build Tools, global MinGW, global Rust, and global Python
installations are not required. Setup installs the pinned Rust toolchain,
repo-local MinGW GNU build support, and a portable Python documentation
toolchain inside the repository-local `.dev` folder.

## First Setup

Run from the repository root:

```bat
.\dev.bat setup
```

Setup performs:

```text
1. create .dev/cargo, .dev/rustup, .dev/python, and .dev/cache
2. download rustup-init.exe into .dev/cache when needed
3. install 1.96.0-x86_64-pc-windows-gnu into .dev
4. install MSYS2/MinGW GNU build support into .dev
5. install portable Python 3.13.5 and Python documentation requirements
6. fetch locked Cargo dependencies
7. build the Rust workspace
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
EngLang 0.1.0-preview

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
target\debug\eng.exe run examples\official\01_csv_plot\main.eng
target\debug\eng.exe run examples\official\02_simple_system\main.eng --save-artifacts
```

The first command keeps artifacts in memory. Add `--save-artifacts` when you
want files. Saved artifacts:

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
.\dev.bat ide --smoke
.\dev.bat artifacts-check
.\dev.bat package
.\dev.bat package-smoke
.\dev.bat clean
```

`package-smoke` builds the portable zip, extracts it into a path containing
spaces and Korean characters, and runs the packaged `eng.exe` plus
`eng-ide.exe --smoke` without relying on Rust or Python in the target folder.
It also builds a standalone bundle and runs that bundle's `run.bat`.

## Native Tester IDE

During development:

```bat
.\dev.bat ide
```

From a portable release package:

```bat
eng-ide.exe
```

The native tester IDE supports file browsing, new file creation, source editing,
syntax highlighting, live compiler diagnostics, completion insertion, symbol
metadata, running the current file, PlotSpec preview, and artifact opening. See
[Native tester IDE](../guide/native_ide.md).

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
