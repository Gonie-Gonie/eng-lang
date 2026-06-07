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

function Get-WorkspaceVersion {
    $inWorkspacePackage = $false
    foreach ($line in Get-Content (Join-Path $RepoRoot "Cargo.toml")) {
        $trimmed = $line.Trim()
        if ($trimmed -eq "[workspace.package]") {
            $inWorkspacePackage = $true
            continue
        }
        if ($inWorkspacePackage -and $trimmed.StartsWith("[")) {
            break
        }
        if ($inWorkspacePackage -and $line -match '^\s*version\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }
    throw "workspace package version not found in Cargo.toml"
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
    $Version = Get-WorkspaceVersion
    $PackageRoot = Join-Path $RepoRoot "dist\englang-preview"
    $ZipPath = Join-Path $RepoRoot "dist\englang-preview-v$Version-windows-x64.zip"
    $ChecksumPath = "$ZipPath.sha256"
    Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ZipPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng.exe") (Join-Path $PackageRoot "eng.exe")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "examples") (Join-Path $PackageRoot "examples")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "stdlib") (Join-Path $PackageRoot "stdlib")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "docs") (Join-Path $PackageRoot "docs")
    Set-Content -Path (Join-Path $PackageRoot "README.txt") -Encoding ascii -Value @"
EngLang portable package

This folder is self-contained for preview execution. Rust and Python are not
required on the target PC.

Recommended smoke commands:
  eng.exe doctor
  eng.exe run examples\04_plotting\main.eng --entry main
  eng.exe run examples\06_simple_system\main.eng --entry main
  eng.exe build examples\02_csv_plot\main.eng --entry main --standalone --profile repro
  dist\main-standalone\run.bat
  eng.exe view build\result\result.engres

Generated artifacts are written under build\result in the current folder.
"@
    Compress-Archive -Path (Join-Path $PackageRoot "*") -DestinationPath $ZipPath -Force
    $Hash = Get-FileHash -Algorithm SHA256 $ZipPath
    "$($Hash.Hash.ToLowerInvariant())  $(Split-Path -Leaf $ZipPath)" | Set-Content -Path $ChecksumPath -Encoding ascii -NoNewline
    Write-Host "Package prepared at $PackageRoot"
    Write-Host "Zip prepared at $ZipPath"
    Write-Host "Checksum prepared at $ChecksumPath"
}

function Invoke-PackageSmoke {
    Invoke-Package
    $Version = Get-WorkspaceVersion
    $ZipPath = Join-Path $RepoRoot "dist\englang-preview-v$Version-windows-x64.zip"
    $KoreanWord = -join @([char]0xD55C, [char]0xAE00)
    $SmokeRoot = Join-Path $RepoRoot "dist\portable smoke $KoreanWord"
    Remove-Item -LiteralPath $SmokeRoot -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $SmokeRoot | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $SmokeRoot -Force
    $Eng = Join-Path $SmokeRoot "eng.exe"

    Push-Location $SmokeRoot
    try {
        Invoke-Native $Eng "doctor"
        Invoke-Native $Eng "run" "examples\04_plotting\main.eng" "--entry" "main"
        Invoke-Native $Eng "view" "build\result\result.engres"
        Invoke-Native $Eng "run" "examples\06_simple_system\main.eng" "--entry" "main"
        if (-not (Test-Path (Join-Path $SmokeRoot "build\result\report_spec.json"))) {
            throw "portable smoke did not create build\result\report_spec.json"
        }
        Invoke-Native $Eng "build" "examples\02_csv_plot\main.eng" "--entry" "main" "--standalone" "--profile" "repro"
        $StandaloneRunner = Join-Path $SmokeRoot "dist\main-standalone\run.bat"
        if (-not (Test-Path $StandaloneRunner)) {
            throw "portable smoke did not create dist\main-standalone\run.bat"
        }
        Invoke-Native $StandaloneRunner
        if (-not (Test-Path (Join-Path $SmokeRoot "dist\main-standalone\build\result\plots\plot_spec.json"))) {
            throw "standalone packaged runner did not create PlotSpec artifacts"
        }
    } finally {
        Pop-Location
    }

    Write-Host "Portable package smoke passed at $SmokeRoot"
}

function Invoke-ReleaseCheck {
    Invoke-Ci
    Invoke-PackageSmoke
    $Version = Get-WorkspaceVersion
    $ZipPath = Join-Path $RepoRoot "dist\englang-preview-v$Version-windows-x64.zip"
    $ChecksumPath = "$ZipPath.sha256"
    if (-not (Test-Path $ZipPath)) {
        throw "release check did not create $ZipPath"
    }
    if (-not (Test-Path $ChecksumPath)) {
        throw "release check did not create $ChecksumPath"
    }
    $ExpectedHash = (Get-Content -LiteralPath $ChecksumPath -Raw).Split(" ")[0].Trim()
    $ActualHash = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLowerInvariant()
    if ($ExpectedHash -ne $ActualHash) {
        throw "release checksum mismatch for $ZipPath"
    }
    $ManifestPath = Join-Path $RepoRoot "dist\release-manifest.txt"
    $GitCommit = try {
        (& git rev-parse --short HEAD 2>$null)
    } catch {
        "unknown"
    }
    Set-Content -Path $ManifestPath -Encoding ascii -Value @"
EngLang release check

version = $Version
commit = $GitCommit
zip = $(Split-Path -Leaf $ZipPath)
sha256 = $ActualHash

verified:
  dev.bat ci
  dev.bat package-smoke
  standalone packaged runner
"@
    Write-Host "Release check passed."
    Write-Host "Manifest prepared at $ManifestPath"
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
  .\dev.bat package        Build release, assemble dist\englang-preview, zip it, and write SHA256
  .\dev.bat package-smoke  Extract the portable zip under a Korean/space path and smoke it
  .\dev.bat release-check  Run full local release gate and verify checksum
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
    "package-smoke" { Invoke-PackageSmoke }
    "release-check" { Invoke-ReleaseCheck }
    "clean" { Invoke-Clean }
    default { Show-Help }
}
