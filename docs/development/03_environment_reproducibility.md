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
4. setup installs the toolchain and MinGW build support under repo-local .dev.
5. rust-toolchain.toml and scripts/dev.ps1 must agree on the pinned toolchain.
6. Portable Python is allowed for development-time documentation generation.
7. The core run/report/plot path must not require Python.
8. The packaged preview path must not require Rust or Python on the target PC.
9. CI and local checks should use the same dev.bat commands.
```

## Repo-Local Toolchain

`scripts/dev.ps1` sets:

```text
CARGO_HOME  = <repo>\.dev\cargo
RUSTUP_HOME = <repo>\.dev\rustup
PATH        = <repo>\.dev\msys64\mingw64\bin;<repo>\.dev\cargo\bin;<repo>\.dev\python;<repo>\.dev\python\Scripts;%PATH%
ENG_REPO_ROOT = <repo>
```

If a global Rust installation exists, the wrapper can use it as a fallback, but
the preferred and documented path is the repository-local toolchain.

## Repo-Local MinGW

Setup installs MSYS2 base plus the `mingw-w64-x86_64-gcc` package under:

```text
.dev/msys64
```

This provides GNU build tools such as `dlltool.exe`, `as.exe`, import
libraries, and GCC support needed by Windows GUI dependencies in the IDE
workspace. Do not rely on a global MSYS2 or MinGW installation for local
validation.

## Pinned Rust

Current pin:

```text
1.96.0-x86_64-pc-windows-gnu
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
3. confirm Python dependencies stay in tools/python/requirements.txt
4. confirm the package path still has no Rust/Python target dependency
5. update docs when artifact formats or public behavior change
6. consider cargo vendor only when reproducibility or security needs justify it
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
eng-ide.exe
examples/
stdlib/
docs/
tools/
README.txt
```

`docs/` in the package is curated release documentation, not a copy of the
developer markdown tree. `README.txt` inside the package gives target-PC smoke
commands.

## Standalone Bundle

Build a runnable model bundle:

```bat
target\debug\eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
dist\main-standalone\run.bat
```

The bundle contains:

```text
eng.exe
run.bat
ARGS_HELP.txt
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
9. runs the official integrated HVAC user-test example
10. verifies integrated HVAC policy, solver, and plot artifacts
11. builds a standalone bundle from the packaged eng.exe
12. runs the standalone bundle's run.bat
13. verifies the standalone bundle creates PlotSpec artifacts
14. verifies packaged docs contain the curated PDF and no developer markdown
```

The smoke folder intentionally contains both a space and Korean characters. This
guards against path handling bugs before a preview package is shared.

## Clean Rebuild

```bat
.\dev.bat clean
.\dev.bat setup
.\dev.bat release-check
```

This is the strongest local release sanity check before tagging.
