# Environment Reproducibility Policy

EngLang is designed around a Windows portable preview package and a
repository-local development toolchain. The core rule is simple: cloning the
repository and running `dev.bat setup` should make the same development
environment available on every supported PC.

## Policy

```text
1. The repository root dev.bat is the only development entry point.
2. PowerShell implementation lives in scripts/dev.ps1.
3. dev.bat always calls PowerShell with ExecutionPolicy Bypass.
4. setup installs the toolchain under repo-local .dev.
5. rust-toolchain.toml and scripts/dev.ps1 must agree on the pinned toolchain.
6. The core run/report/plot path must not require Python.
7. The packaged preview path must not require Rust or Python on the target PC.
8. CI and local checks should use the same dev.bat commands.
```

## Repo-Local Toolchain

`scripts/dev.ps1` sets:

```text
CARGO_HOME  = <repo>\.dev\cargo
RUSTUP_HOME = <repo>\.dev\rustup
PATH        = <repo>\.dev\cargo\bin;%PATH%
ENG_REPO_ROOT = <repo>
```

If a global Rust installation exists, the wrapper can use it as a fallback, but
the preferred and documented path is the repository-local toolchain.

## Pinned Rust

Current pin:

```text
1.78.0-x86_64-pc-windows-gnu
```

Reasons:

```text
- avoids requiring Visual Studio Build Tools for the preview path
- keeps compiler behavior consistent across Windows PCs
- supports the current Rust 2021 implementation
```

When changing the pin, update:

```text
rust-toolchain.toml
scripts/dev.ps1 $PinnedToolchain
docs/development/03_environment_reproducibility.md
release notes for the active milestone
```

## Dependency Policy

Rust dependencies are currently intentionally minimal. When adding dependencies:

```text
1. commit Cargo.lock
2. confirm the core path still has no Python dependency
3. confirm the package path still has no Rust/Python target dependency
4. update docs when artifact formats or public behavior change
5. consider cargo vendor only when reproducibility or security needs justify it
```

Future vendoring structure, if adopted:

```text
vendor/
.cargo/config.toml
Cargo.lock
```

## Build Artifact Policy

Do not commit:

```text
.dev/
target/
build/
dist/
*.engbc
*.engres
```

Commit:

```text
source code
stdlib source
examples
docs
Cargo.lock
toolchain/config scripts
```

## Portable Packaging

Build a portable package:

```bat
.\dev.bat package
```

This creates:

```text
dist\englang-preview\
dist\englang-preview-v<version>-windows-x64.zip
dist\englang-preview-v<version>-windows-x64.zip.sha256
```

The unpacked package contains:

```text
eng.exe
examples/
stdlib/
docs/
README.txt
```

`README.txt` inside the package gives target-PC smoke commands.

## Standalone Bundle

Build a runnable model bundle:

```bat
target\debug\eng.exe build examples\02_csv_plot\main.eng --entry main --standalone --profile repro
dist\main-standalone\run.bat
```

The bundle contains:

```text
eng.exe
run.bat
main.engbc
main.engpkg
main.lock
main.review.html
source/main.eng
source/data/sensor.csv
```

`run.bat` writes normal `build/result` artifacts inside the bundle.

## Portable Smoke

Run:

```bat
.\dev.bat package-smoke
```

This command:

```text
1. builds the release binary
2. assembles dist\englang-preview
3. writes the portable zip and SHA256 checksum
4. extracts the zip under dist\portable smoke <Korean word>
5. runs packaged eng.exe doctor from the extracted folder
6. runs the official CSV+plot example
7. runs eng.exe view on the generated result
8. runs the official simple system example
9. verifies build\result\report_spec.json exists
10. builds a standalone bundle from the packaged eng.exe
11. runs the standalone bundle's run.bat
12. verifies the standalone bundle creates PlotSpec artifacts
```

The smoke folder intentionally contains both a space and Korean characters. This
guards against path handling bugs before a preview package is shared.

## Clean Rebuild

```bat
.\dev.bat clean
.\dev.bat setup
.\dev.bat ci
.\dev.bat package-smoke
```

This is the strongest local release sanity check before tagging.
