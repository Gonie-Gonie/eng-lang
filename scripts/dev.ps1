param(
    [Parameter(Position = 0)]
    [string] $Command = "help",

    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]] $Rest
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$DevHome = Join-Path $RepoRoot ".dev"
$CargoHome = Join-Path $DevHome "cargo"
$RustupHome = Join-Path $DevHome "rustup"
$CacheHome = Join-Path $DevHome "cache"
$RustupInit = Join-Path $CacheHome "rustup-init.exe"
$RustupUrl = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
$PinnedToolchain = "1.78.0-x86_64-pc-windows-gnu"

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true, Position = 0)]
        [string] $FilePath,

        [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
        [string[]] $Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

function Set-DevEnvironment {
    New-Item -ItemType Directory -Force -Path $CargoHome, $RustupHome, $CacheHome | Out-Null
    $env:CARGO_HOME = $CargoHome
    $env:RUSTUP_HOME = $RustupHome
    $env:PATH = "$CargoHome\bin;$env:PATH"
    $env:ENG_REPO_ROOT = $RepoRoot
}

function Get-Cargo {
    Set-DevEnvironment
    $cargo = Join-Path $CargoHome "bin\cargo.exe"
    if (Test-Path $cargo) {
        return $cargo
    }
    $globalCargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($null -ne $globalCargo) {
        return $globalCargo.Source
    }
    return $null
}

function Invoke-Setup {
    Set-DevEnvironment
    if (-not (Test-Path (Join-Path $CargoHome "bin\cargo.exe"))) {
        if (-not (Test-Path $RustupInit)) {
            Write-Host "Downloading rustup-init into .dev cache..."
            Invoke-WebRequest -Uri $RustupUrl -OutFile $RustupInit
        }
        Write-Host "Installing pinned Rust toolchain into .dev..."
        Invoke-Native $RustupInit "-y" "--no-modify-path" "--profile" "minimal" "--default-toolchain" $PinnedToolchain
    }

    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        throw "Cargo was not found after setup."
    }

    if (-not (Test-Path (Join-Path $RepoRoot "Cargo.lock"))) {
        Write-Host "Generating Cargo.lock..."
        Invoke-Native $cargo "generate-lockfile"
    }
    Write-Host "Fetching locked dependencies..."
    Invoke-Native $cargo "fetch" "--locked"
    Write-Host "Building workspace..."
    Invoke-Native $cargo "build" "--workspace"
    Write-Host "Setup complete. Use .\dev.bat doctor next."
}

function Invoke-Doctor {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "doctor"
}

function Invoke-Build {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "--workspace"
}

function Invoke-Test {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "test" "--workspace"
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "test" "examples"
}

function Invoke-Fmt {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "fmt" "--all"
}

function Invoke-Clippy {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "clippy" "--workspace" "--all-targets" "--" "-D" "warnings"
}

function Invoke-Ci {
    Invoke-Fmt
    Invoke-Test
    Invoke-Clippy
    Invoke-RunExample
}

function Invoke-RunExample {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    $example = if ($Rest.Count -gt 0) { $Rest[0] } else { "examples\04_plotting\main.eng" }
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "run" $example
}

function Invoke-Package {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "--workspace" "--release"
    $PackageRoot = Join-Path $RepoRoot "dist\englang-preview"
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng.exe") (Join-Path $PackageRoot "eng.exe")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "examples") (Join-Path $PackageRoot "examples")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "stdlib") (Join-Path $PackageRoot "stdlib")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "docs") (Join-Path $PackageRoot "docs")
    Write-Host "Package prepared at $PackageRoot"
}

function Invoke-Clean {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -ne $cargo) {
        Invoke-Native $cargo "clean"
    }
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $RepoRoot "build")
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $RepoRoot "dist")
}

function Show-Help {
    Write-Host @"
EngLang development wrapper

Usage:
  .\dev.bat setup          Install pinned local Rust toolchain in .dev and build
  .\dev.bat doctor         Run eng doctor through the local toolchain
  .\dev.bat build          Build the Rust workspace
  .\dev.bat test           Run Rust tests and EngLang example smoke tests
  .\dev.bat fmt            Format Rust code
  .\dev.bat clippy         Run clippy with warnings denied
  .\dev.bat ci             Run fmt, tests, clippy, and preview example
  .\dev.bat run-example    Run examples\04_plotting\main.eng
  .\dev.bat package        Build release and assemble dist\englang-preview
  .\dev.bat clean          Remove build artifacts

All PowerShell execution goes through dev.bat with ExecutionPolicy Bypass.
"@
}

Set-Location $RepoRoot

switch ($Command) {
    "setup" { Invoke-Setup }
    "doctor" { Invoke-Doctor }
    "build" { Invoke-Build }
    "test" { Invoke-Test }
    "fmt" { Invoke-Fmt }
    "clippy" { Invoke-Clippy }
    "ci" { Invoke-Ci }
    "run-example" { Invoke-RunExample }
    "package" { Invoke-Package }
    "clean" { Invoke-Clean }
    default { Show-Help }
}
