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

function Get-CodeFences {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    $lines = Get-Content -LiteralPath $Path -Encoding UTF8
    $inFence = $false
    $info = ""
    $startLine = 0
    $body = New-Object System.Collections.Generic.List[string]

    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]
        if ($line -match '^```(.*)$') {
            if (-not $inFence) {
                $inFence = $true
                $info = $Matches[1].Trim()
                $startLine = $index + 1
                $body.Clear()
            } else {
                [pscustomobject]@{
                    File = $Path
                    StartLine = $startLine
                    Info = $info
                    Body = ($body -join [Environment]::NewLine)
                }
                $inFence = $false
                $info = ""
                $startLine = 0
                $body.Clear()
            }
        } elseif ($inFence) {
            $body.Add($line) | Out-Null
        }
    }

    if ($inFence) {
        throw "Unclosed code fence in $Path starting at line $startLine"
    }
}

function Invoke-DocsCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "-p" "eng_cli"
    $Eng = Join-Path $RepoRoot "target\debug\eng.exe"
    $DocsCheckRoot = Join-Path $RepoRoot "build\docs-check"
    $Utf8NoBom = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false
    Remove-Item -LiteralPath $DocsCheckRoot -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $DocsCheckRoot | Out-Null

    $targets = @(
        "README.md",
        "docs\specs",
        "docs\reference",
        "docs\guide",
        "docs\tutorials",
        "docs\architecture",
        "docs\runtime"
    )
    $markdownFiles = New-Object System.Collections.Generic.List[string]
    foreach ($target in $targets) {
        $path = Join-Path $RepoRoot $target
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $markdownFiles.Add($path) | Out-Null
        } elseif (Test-Path -LiteralPath $path -PathType Container) {
            Get-ChildItem -LiteralPath $path -Recurse -Filter "*.md" | ForEach-Object {
                $markdownFiles.Add($_.FullName) | Out-Null
            }
        }
    }

    $checked = 0
    $skipped = 0
    $snippetIndex = 0
    foreach ($file in $markdownFiles) {
        foreach ($fence in (Get-CodeFences -Path $file)) {
            $info = $fence.Info.ToLowerInvariant()
            if (-not $info.StartsWith("eng")) {
                continue
            }
            if ($info -match '\b(future|partial|unchecked)\b') {
                $skipped += 1
                continue
            }
            $expectFailure = $info -match '\b(error|fail)\b'
            $snippetIndex += 1
            $safeName = $file.Substring($RepoRoot.Length).TrimStart('\') -replace '[\\/:*?"<>| ]', '_'
            $snippetPath = Join-Path $DocsCheckRoot ("{0:D4}_{1}.eng" -f $snippetIndex, $safeName)
            [System.IO.File]::WriteAllText($snippetPath, $fence.Body, $Utf8NoBom)

            & $Eng "check" $snippetPath
            $exitCode = $LASTEXITCODE
            if ($expectFailure) {
                if ($exitCode -eq 0) {
                    throw "Docs snippet was expected to fail but passed: $($fence.File):$($fence.StartLine)"
                }
                if ($exitCode -ne 2) {
                    throw "Docs snippet failed with unexpected exit code $exitCode`: $($fence.File):$($fence.StartLine)"
                }
            } else {
                if ($exitCode -ne 0) {
                    throw "Docs snippet failed: $($fence.File):$($fence.StartLine)"
                }
            }
            $checked += 1
        }
    }

    Write-Host "Docs check passed. Checked $checked Eng snippet(s), skipped $skipped marked snippet(s)."
}

function Assert-Artifact {
    param(
        [Parameter(Mandatory = $true)]
        [bool] $Condition,

        [Parameter(Mandatory = $true)]
        [string] $Message
    )

    if (-not $Condition) {
        throw "Artifact check failed: $Message"
    }
}

function Assert-ArtifactValue {
    param(
        [Parameter(Mandatory = $true)]
        $Actual,

        [Parameter(Mandatory = $true)]
        $Expected,

        [Parameter(Mandatory = $true)]
        [string] $Label
    )

    Assert-Artifact ([string]$Actual -eq [string]$Expected) "$Label expected $Expected but got $Actual"
}

function Assert-ArtifactNumber {
    param(
        [Parameter(Mandatory = $true)]
        $Actual,

        [Parameter(Mandatory = $true)]
        [int] $Expected,

        [Parameter(Mandatory = $true)]
        [string] $Label
    )

    Assert-Artifact ([int]$Actual -eq $Expected) "$Label expected $Expected but got $Actual"
}

function Read-ArtifactJson {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    Assert-Artifact (Test-Path -LiteralPath $Path -PathType Leaf) "missing JSON artifact $Path"
    return Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Read-KeyValueArtifact {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    Assert-Artifact (Test-Path -LiteralPath $Path -PathType Leaf) "missing key/value artifact $Path"
    $values = @{}
    foreach ($line in Get-Content -LiteralPath $Path -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed.Length -eq 0 -or $trimmed.StartsWith("#")) {
            continue
        }
        $parts = $trimmed -split "\s*=\s*", 2
        if ($parts.Count -eq 2) {
            $values[$parts[0]] = $parts[1]
        }
    }
    return $values
}

function Get-NormalizedArtifactPath {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Value
    )

    return $Value.Replace("/", "\")
}

function Assert-SchemaFilesPresent {
    $schemaFiles = @(
        "docs\schemas\review.schema.json",
        "docs\schemas\report_spec.schema.json",
        "docs\schemas\result.schema.json",
        "docs\schemas\plotspec.schema.json",
        "docs\schemas\engpkg.schema.json"
    )

    foreach ($schemaFile in $schemaFiles) {
        $path = Join-Path $RepoRoot $schemaFile
        Assert-Artifact (Test-Path -LiteralPath $path -PathType Leaf) "missing schema file $schemaFile"
        $schema = Read-ArtifactJson $path
        Assert-Artifact ([string]$schema.'$schema' -ne "") "$schemaFile does not declare a JSON schema dialect"
        Assert-Artifact ([string]$schema.title -ne "") "$schemaFile does not declare a title"
    }
}

function Assert-CsvPlotGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--entry" "main"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "review.review_schema_version"
    Assert-ArtifactValue (Get-NormalizedArtifactPath $review.source_path) (Get-NormalizedArtifactPath $Golden.source) "review.source_path"
    Assert-ArtifactNumber $review.syntax_summary.scripts $Golden.review.scripts "review.syntax_summary.scripts"
    Assert-ArtifactNumber $review.syntax_summary.schemas $Golden.review.schemas "review.syntax_summary.schemas"
    Assert-ArtifactNumber $review.syntax_summary.systems $Golden.review.systems "review.syntax_summary.systems"
    Assert-ArtifactNumber $review.syntax_summary.equations $Golden.review.equations "review.syntax_summary.equations"
    Assert-ArtifactNumber @($review.schema_summary).Count $Golden.review.schema_summary_count "review.schema_summary count"
    Assert-ArtifactNumber @($review.csv_promotions).Count $Golden.review.csv_promotion_count "review.csv_promotions count"

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "report_spec.report_schema_version"
    Assert-ArtifactNumber $reportSpec.provenance.schema_count $Golden.report_spec.schema_count "report_spec.provenance.schema_count"
    Assert-ArtifactNumber $reportSpec.provenance.csv_promotion_count $Golden.report_spec.csv_promotion_count "report_spec.provenance.csv_promotion_count"
    Assert-ArtifactNumber $reportSpec.provenance.system_count $Golden.report_spec.system_count "report_spec.provenance.system_count"
    Assert-ArtifactNumber $reportSpec.provenance.residual_count $Golden.report_spec.residual_count "report_spec.provenance.residual_count"
    Assert-ArtifactNumber $reportSpec.provenance.plot_spec_version $Golden.report_spec.plot_spec_version "report_spec.provenance.plot_spec_version"

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "result.result_format_version"
    Assert-ArtifactNumber $result.bytecode_version $Golden.result.bytecode_version "result.bytecode_version"
    Assert-ArtifactValue $result.entry.name $Golden.result.entry_name "result.entry.name"
    Assert-ArtifactNumber $result.object_store.table_count $Golden.result.table_count "result.object_store.table_count"
    Assert-ArtifactNumber $result.object_store.timeseries_count $Golden.result.timeseries_count "result.object_store.timeseries_count"
    Assert-ArtifactNumber $result.provenance.schema_count $Golden.result.schema_count "result.provenance.schema_count"
    Assert-ArtifactNumber $result.provenance.csv_promotion_count $Golden.result.csv_promotion_count "result.provenance.csv_promotion_count"
    Assert-ArtifactNumber @($result.typed_payload.statistics).Count $Golden.result.statistics_count "result.typed_payload.statistics count"
    Assert-ArtifactNumber @($result.typed_payload.integrations).Count $Golden.result.integrations_count "result.typed_payload.integrations count"

    $plotSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\plots\plot_spec.json")
    Assert-ArtifactValue $plotSpec.format $Golden.plot_spec.format "plot_spec.format"
    Assert-ArtifactNumber $plotSpec.plot_spec_version $Golden.plot_spec.plot_spec_version "plot_spec.plot_spec_version"
    Assert-ArtifactValue $plotSpec.plot_type $Golden.plot_spec.plot_type "plot_spec.plot_type"
    Assert-ArtifactNumber @($plotSpec.series).Count $Golden.plot_spec.series_count "plot_spec.series count"
    $firstSeries = @($plotSpec.series)[0]
    Assert-ArtifactValue $firstSeries.name $Golden.plot_spec.first_series "plot_spec.series[0].name"
    Assert-ArtifactNumber @($firstSeries.points).Count $Golden.plot_spec.point_count "plot_spec.series[0].points count"

    Remove-Item -LiteralPath (Join-Path $RepoRoot "dist\main-standalone") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "build" $Golden.source "--entry" "main" "--standalone" "--profile" "repro"
    $engpkg = Read-KeyValueArtifact (Join-Path $RepoRoot "dist\main-standalone\main.engpkg")
    Assert-ArtifactValue $engpkg["format"] $Golden.engpkg.format "engpkg.format"
    Assert-ArtifactValue $engpkg["package_format_version"] $Golden.engpkg.package_format_version "engpkg.package_format_version"
    Assert-ArtifactValue $engpkg["runner"] $Golden.engpkg.runner "engpkg.runner"
    Assert-ArtifactValue $engpkg["engine"] $Golden.engpkg.engine "engpkg.engine"
    Assert-ArtifactValue $engpkg["source"] $Golden.engpkg.source "engpkg.source"
    Assert-ArtifactValue $engpkg["bytecode"] $Golden.engpkg.bytecode "engpkg.bytecode"
    Assert-ArtifactValue $engpkg["entry_name"] $Golden.engpkg.entry_name "engpkg.entry_name"

    $lock = Read-KeyValueArtifact (Join-Path $RepoRoot "dist\main-standalone\main.lock")
    Assert-ArtifactValue $lock["bytecode_version"] "1" "lock.bytecode_version"
    Assert-ArtifactValue $lock["result_format_version"] "1" "lock.result_format_version"
    Assert-ArtifactValue $lock["report_schema_version"] "1" "lock.report_schema_version"
    Assert-ArtifactValue $lock["plot_spec_version"] "1" "lock.plot_spec_version"
}

function Assert-SystemGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--entry" "main"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "system review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "system review.review_schema_version"
    Assert-ArtifactNumber $review.syntax_summary.scripts $Golden.review.scripts "system review.syntax_summary.scripts"
    Assert-ArtifactNumber $review.syntax_summary.schemas $Golden.review.schemas "system review.syntax_summary.schemas"
    Assert-ArtifactNumber $review.syntax_summary.systems $Golden.review.systems "system review.syntax_summary.systems"
    Assert-ArtifactNumber $review.syntax_summary.equations $Golden.review.equations "system review.syntax_summary.equations"
    Assert-ArtifactNumber @($review.system_summary).Count $Golden.review.system_summary_count "system review.system_summary count"
    Assert-ArtifactNumber @(@($review.system_summary)[0].residuals).Count $Golden.review.residual_count "system review residual count"

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "system report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "system report_spec.report_schema_version"
    Assert-ArtifactNumber $reportSpec.provenance.schema_count $Golden.report_spec.schema_count "system report_spec.provenance.schema_count"
    Assert-ArtifactNumber $reportSpec.provenance.csv_promotion_count $Golden.report_spec.csv_promotion_count "system report_spec.provenance.csv_promotion_count"
    Assert-ArtifactNumber $reportSpec.provenance.system_count $Golden.report_spec.system_count "system report_spec.provenance.system_count"
    Assert-ArtifactNumber $reportSpec.provenance.equation_count $Golden.report_spec.equation_count "system report_spec.provenance.equation_count"
    Assert-ArtifactNumber $reportSpec.provenance.residual_count $Golden.report_spec.residual_count "system report_spec.provenance.residual_count"

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "system result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "system result.result_format_version"
    Assert-ArtifactNumber $result.bytecode_version $Golden.result.bytecode_version "system result.bytecode_version"
    Assert-ArtifactValue $result.entry.name $Golden.result.entry_name "system result.entry.name"
    Assert-ArtifactNumber $result.object_store.table_count $Golden.result.table_count "system result.object_store.table_count"
    Assert-ArtifactNumber $result.object_store.timeseries_count $Golden.result.timeseries_count "system result.object_store.timeseries_count"
    Assert-ArtifactNumber $result.provenance.system_count $Golden.result.system_count "system result.provenance.system_count"
    Assert-ArtifactNumber $result.provenance.equation_count $Golden.result.equation_count "system result.provenance.equation_count"
    Assert-ArtifactNumber $result.provenance.residual_count $Golden.result.residual_count "system result.provenance.residual_count"
}

function Invoke-ArtifactsCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "-p" "eng_cli"
    $Eng = Join-Path $RepoRoot "target\debug\eng.exe"

    Assert-SchemaFilesPresent
    $goldenRoot = Join-Path $RepoRoot "tests\golden\artifacts"
    $csvGolden = Read-ArtifactJson (Join-Path $goldenRoot "official_01_csv_plot.golden.json")
    $systemGolden = Read-ArtifactJson (Join-Path $goldenRoot "official_02_simple_system.golden.json")

    Assert-CsvPlotGolden $csvGolden $Eng
    Assert-SystemGolden $systemGolden $Eng

    Write-Host "Artifact check passed. Validated schema files and official golden artifacts."
}

function Invoke-RunExample {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    $example = if ($Rest.Count -gt 0) { $Rest[0] } else { "examples\official\01_csv_plot\main.eng" }
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
  eng.exe run examples\official\01_csv_plot\main.eng --entry main
  eng.exe run examples\official\02_simple_system\main.eng --entry main
  eng.exe build examples\official\01_csv_plot\main.eng --entry main --standalone --profile repro
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
        Invoke-Native $Eng "run" "examples\official\01_csv_plot\main.eng" "--entry" "main"
        Invoke-Native $Eng "view" "build\result\result.engres"
        Invoke-Native $Eng "run" "examples\official\02_simple_system\main.eng" "--entry" "main"
        if (-not (Test-Path (Join-Path $SmokeRoot "build\result\report_spec.json"))) {
            throw "portable smoke did not create build\result\report_spec.json"
        }
        Invoke-Native $Eng "build" "examples\official\01_csv_plot\main.eng" "--entry" "main" "--standalone" "--profile" "repro"
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
    Invoke-DocsCheck
    Invoke-ArtifactsCheck
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
  dev.bat docs-check
  dev.bat artifacts-check
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
  .\dev.bat docs-check     Check supported documentation Eng snippets
  .\dev.bat artifacts-check Validate artifact schemas and golden baselines
  .\dev.bat run-example    Run examples\official\01_csv_plot\main.eng
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
    "docs-check" { Invoke-DocsCheck }
    "artifacts-check" { Invoke-ArtifactsCheck }
    "run-example" { Invoke-RunExample }
    "package" { Invoke-Package }
    "package-smoke" { Invoke-PackageSmoke }
    "release-check" { Invoke-ReleaseCheck }
    "clean" { Invoke-Clean }
    default { Show-Help }
}
