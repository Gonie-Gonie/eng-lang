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
$PinnedToolchain = "1.96.0-x86_64-pc-windows-gnu"
$MsysVersion = "20241116"
$MsysHome = Join-Path $DevHome "msys64"
$MingwBin = Join-Path $MsysHome "mingw64\bin"
$MsysBash = Join-Path $MsysHome "usr\bin\bash.exe"
$MsysArchive = Join-Path $CacheHome "msys2-base-x86_64-$MsysVersion.tar.xz"
$MsysUrl = "https://repo.msys2.org/distrib/x86_64/msys2-base-x86_64-$MsysVersion.tar.xz"
$MingwPackage = "mingw-w64-x86_64-gcc"
$PythonVersion = "3.13.5"
$PythonHome = Join-Path $DevHome "python"
$PythonZip = Join-Path $CacheHome "python-$PythonVersion-embed-amd64.zip"
$PythonUrl = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
$GetPipPath = Join-Path $CacheHome "get-pip.py"
$GetPipUrl = "https://bootstrap.pypa.io/get-pip.py"
$PythonRequirements = Join-Path $RepoRoot "tools\python\requirements.txt"

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
    $env:PATH = "$MingwBin;$CargoHome\bin;$PythonHome;$PythonHome\Scripts;$env:PATH"
    $env:ENG_REPO_ROOT = $RepoRoot
    $env:PYTHONUTF8 = "1"
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

function Get-PortablePython {
    Set-DevEnvironment
    $python = Join-Path $PythonHome "python.exe"
    if (Test-Path $python) {
        return $python
    }
    return $null
}

function Enable-EmbeddedPythonSitePackages {
    $pth = Get-ChildItem -LiteralPath $PythonHome -Filter "python*._pth" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $pth) {
        return
    }

    $lines = Get-Content -LiteralPath $pth.FullName -Encoding ascii
    $updated = New-Object System.Collections.Generic.List[string]
    $hasSitePackages = $false
    $hasImportSite = $false
    foreach ($line in $lines) {
        if ($line.Trim() -eq "Lib\site-packages") {
            $hasSitePackages = $true
        }
        if ($line.Trim() -eq "import site") {
            $hasImportSite = $true
            $updated.Add("import site") | Out-Null
        } elseif ($line.Trim() -eq "#import site") {
            $hasImportSite = $true
            $updated.Add("import site") | Out-Null
        } else {
            $updated.Add($line) | Out-Null
        }
    }
    if (-not $hasSitePackages) {
        $updated.Add("Lib\site-packages") | Out-Null
    }
    if (-not $hasImportSite) {
        $updated.Add("import site") | Out-Null
    }
    Set-Content -LiteralPath $pth.FullName -Encoding ascii -Value $updated
}

function Invoke-PortablePythonSetup {
    Set-DevEnvironment
    $python = Join-Path $PythonHome "python.exe"
    if (-not (Test-Path $python)) {
        if (-not (Test-Path $PythonZip)) {
            Write-Host "Downloading portable Python $PythonVersion into .dev cache..."
            Invoke-WebRequest -Uri $PythonUrl -OutFile $PythonZip
        }
        Write-Host "Installing portable Python into .dev..."
        Remove-Item -LiteralPath $PythonHome -Recurse -Force -ErrorAction SilentlyContinue
        New-Item -ItemType Directory -Force -Path $PythonHome | Out-Null
        Expand-Archive -Path $PythonZip -DestinationPath $PythonHome -Force
        Enable-EmbeddedPythonSitePackages
    } else {
        Enable-EmbeddedPythonSitePackages
    }

    if (-not (Test-Path $GetPipPath)) {
        Write-Host "Downloading get-pip.py into .dev cache..."
        Invoke-WebRequest -Uri $GetPipUrl -OutFile $GetPipPath
    }
    $pipCheck = ""
    $pipExit = 1
    try {
        $pipCheck = & $python -m pip --version 2>$null
        $pipExit = $LASTEXITCODE
    } catch {
        $pipCheck = ""
        $pipExit = 1
    }
    if ($pipExit -ne 0 -or [string]::IsNullOrWhiteSpace($pipCheck)) {
        Write-Host "Installing pip into portable Python..."
        Invoke-Native $python $GetPipPath
    }
    if (Test-Path $PythonRequirements) {
        Write-Host "Installing Python documentation requirements..."
        Invoke-Native $python "-m" "pip" "install" "-r" $PythonRequirements
    }
}

function Test-MingwReady {
    $dlltool = Join-Path $MingwBin "dlltool.exe"
    $gcc = Join-Path $MingwBin "x86_64-w64-mingw32-gcc.exe"
    $shlwapi = Join-Path $MsysHome "mingw64\lib\libshlwapi.a"
    return (Test-Path $dlltool) -and (Test-Path $gcc) -and (Test-Path $shlwapi)
}

function Invoke-MingwSetup {
    Set-DevEnvironment
    if (Test-MingwReady) {
        return
    }

    if (-not (Test-Path $MsysBash)) {
        if (-not (Test-Path $MsysArchive)) {
            Write-Host "Downloading MSYS2 base into .dev cache..."
            Invoke-WebRequest -Uri $MsysUrl -OutFile $MsysArchive
        }
        Write-Host "Installing MSYS2 base into .dev..."
        New-Item -ItemType Directory -Force -Path $DevHome | Out-Null
        Invoke-Native "tar.exe" "-xf" $MsysArchive "-C" $DevHome
        if (-not (Test-Path $MsysBash)) {
            throw "MSYS2 bash was not found after extracting $MsysArchive"
        }
        Invoke-Native $MsysBash "-lc" "true"
    }

    Write-Host "Installing MinGW GNU build support into .dev..."
    Invoke-Native $MsysBash "-lc" "pacman -Sy --noconfirm --needed $MingwPackage"
    if (-not (Test-MingwReady)) {
        throw "MinGW GNU build support was not found after installing $MingwPackage"
    }
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

function Get-PublicVersion {
    $Version = Get-WorkspaceVersion
    if ($Version -match '^([0-9]+)\.([0-9]+)\.0-preview$') {
        return "$($Matches[1]).$($Matches[2])-preview"
    }
    return $Version
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
    Invoke-MingwSetup
    Invoke-PortablePythonSetup
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
    Invoke-LspCheck
    Invoke-JitCheck
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

function Assert-ArtifactFloat {
    param(
        [Parameter(Mandatory = $true)]
        $Actual,

        [Parameter(Mandatory = $true)]
        [double] $Expected,

        [Parameter(Mandatory = $true)]
        [string] $Label,

        [double] $Tolerance = 0.000001
    )

    $actualNumber = [double]$Actual
    $delta = [math]::Abs($actualNumber - $Expected)
    Assert-Artifact ($delta -le $Tolerance) "$Label expected $Expected but got $Actual"
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
    Invoke-Native $Eng "run" $Golden.source "--save-artifacts"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "review.review_schema_version"
    Assert-ArtifactValue (Get-NormalizedArtifactPath $review.source_path) (Get-NormalizedArtifactPath $Golden.source) "review.source_path"
    Assert-ArtifactNumber $review.syntax_summary.scripts $Golden.review.scripts "review.syntax_summary.scripts"
    Assert-ArtifactNumber $review.syntax_summary.schemas $Golden.review.schemas "review.syntax_summary.schemas"
    Assert-ArtifactNumber $review.syntax_summary.structs $Golden.review.structs "review.syntax_summary.structs"
    Assert-ArtifactNumber $review.syntax_summary.args_fields $Golden.review.args_fields "review.syntax_summary.args_fields"
    Assert-ArtifactNumber $review.syntax_summary.systems $Golden.review.systems "review.syntax_summary.systems"
    Assert-ArtifactNumber $review.syntax_summary.equations $Golden.review.equations "review.syntax_summary.equations"
    Assert-ArtifactNumber @($review.args_summary).Count $Golden.review.args_block_count "review.args_summary count"
    Assert-ArtifactNumber @(@($review.args_summary)[0].fields).Count $Golden.review.args_field_count "review args field count"
    Assert-ArtifactNumber @($review.arg_values).Count $Golden.review.arg_value_count "review.arg_values count"
    $reviewArgValue = @($review.arg_values)[0]
    Assert-ArtifactValue $reviewArgValue.name $Golden.review.arg_value_name "review.arg_values[0].name"
    Assert-ArtifactValue $reviewArgValue.value $Golden.review.arg_value "review.arg_values[0].value"
    Assert-ArtifactValue $reviewArgValue.source $Golden.review.arg_value_source "review.arg_values[0].source"
    Assert-ArtifactNumber @($review.schema_summary).Count $Golden.review.schema_summary_count "review.schema_summary count"
    Assert-ArtifactNumber @($review.csv_promotions).Count $Golden.review.csv_promotion_count "review.csv_promotions count"
    $reviewPromotion = @($review.csv_promotions)[0]
    Assert-ArtifactValue $reviewPromotion.source_literal $Golden.review.csv_source_literal "review.csv_promotions[0].source_literal"
    Assert-ArtifactValue $reviewPromotion.source_value $Golden.review.csv_source_value "review.csv_promotions[0].source_value"
    $policyWarnings = @(@($review.warning_list) | Where-Object { $_.code -eq "W-SCHEMA-POLICY-001" })
    Assert-ArtifactNumber $policyWarnings.Count $Golden.review.policy_warning_count "review schema policy warning count"

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "report_spec.report_schema_version"
    Assert-ArtifactNumber $reportSpec.provenance.schema_count $Golden.report_spec.schema_count "report_spec.provenance.schema_count"
    Assert-ArtifactNumber $reportSpec.provenance.csv_promotion_count $Golden.report_spec.csv_promotion_count "report_spec.provenance.csv_promotion_count"
    Assert-ArtifactNumber @($reportSpec.args_summary).Count $Golden.report_spec.args_block_count "report_spec.args_summary count"
    Assert-ArtifactNumber @(@($reportSpec.args_summary)[0].fields).Count $Golden.report_spec.args_field_count "report_spec args field count"
    Assert-ArtifactNumber @($reportSpec.arg_values).Count $Golden.report_spec.arg_value_count "report_spec.arg_values count"
    $reportArgValue = @($reportSpec.arg_values)[0]
    Assert-ArtifactValue $reportArgValue.name $Golden.report_spec.arg_value_name "report_spec.arg_values[0].name"
    Assert-ArtifactValue $reportArgValue.value $Golden.report_spec.arg_value "report_spec.arg_values[0].value"
    Assert-ArtifactValue $reportArgValue.source $Golden.report_spec.arg_value_source "report_spec.arg_values[0].source"
    Assert-ArtifactNumber $reportSpec.provenance.system_count $Golden.report_spec.system_count "report_spec.provenance.system_count"
    Assert-ArtifactNumber $reportSpec.provenance.residual_count $Golden.report_spec.residual_count "report_spec.provenance.residual_count"
    Assert-ArtifactNumber $reportSpec.provenance.plot_spec_version $Golden.report_spec.plot_spec_version "report_spec.provenance.plot_spec_version"
    Assert-ArtifactNumber @($reportSpec.computed_statistics).Count $Golden.report_spec.computed_statistics_count "report_spec.computed_statistics count"
    Assert-ArtifactNumber @($reportSpec.computed_integrations).Count $Golden.report_spec.computed_integrations_count "report_spec.computed_integrations count"
    $reportStats = @($reportSpec.computed_statistics)[0]
    Assert-ArtifactValue $reportStats.status $Golden.report_spec.computed_statistics_status "report_spec.computed_statistics[0].status"
    Assert-ArtifactFloat ((@($reportStats.values) | Where-Object { $_.name -eq "time_weighted_mean" }).value) $Golden.report_spec.time_weighted_mean_w "report_spec.time_weighted_mean"
    Assert-ArtifactFloat ((@($reportStats.values) | Where-Object { $_.name -eq "median" }).value) $Golden.report_spec.median_w "report_spec.median"
    Assert-ArtifactFloat ((@($reportStats.values) | Where-Object { $_.name -eq "std" }).value) $Golden.report_spec.std_w "report_spec.std"
    Assert-ArtifactFloat ((@($reportStats.values) | Where-Object { $_.name -eq "p90" }).value) $Golden.report_spec.p90_w "report_spec.p90"
    $reportDurationAbove = @($reportStats.values) | Where-Object { $_.name -eq "duration_above(5 kW)" } | Select-Object -First 1
    Assert-Artifact ($null -ne $reportDurationAbove) "report_spec.computed_statistics missing duration_above(5 kW)"
    Assert-ArtifactFloat $reportDurationAbove.value $Golden.report_spec.duration_above_5kw_s "report_spec.duration_above"
    Assert-ArtifactValue $reportDurationAbove.unit $Golden.report_spec.duration_above_unit "report_spec.duration_above unit"
    $reportIntegration = @($reportSpec.computed_integrations)[0]
    Assert-ArtifactValue $reportIntegration.status $Golden.report_spec.computed_integration_status "report_spec.computed_integrations[0].status"
    Assert-ArtifactFloat $reportIntegration.value $Golden.report_spec.integration_value_j "report_spec.computed_integrations[0].value"
    $reportPolicies = @($reportSpec.policy_results)
    Assert-ArtifactNumber $reportPolicies.Count $Golden.report_spec.policy_result_count "report_spec.policy_results count"
    $reportExecutedPolicies = @($reportPolicies | Where-Object { $_.status -eq "executed" })
    $reportValidatedPolicies = @($reportPolicies | Where-Object { $_.status -eq "validated" })
    Assert-ArtifactNumber $reportExecutedPolicies.Count $Golden.report_spec.policy_executed_count "report_spec.policy_results executed count"
    Assert-ArtifactNumber $reportValidatedPolicies.Count $Golden.report_spec.policy_validated_count "report_spec.policy_results validated count"
    $reportPolicyViolationCount = 0
    foreach ($policy in $reportPolicies) {
        $reportPolicyViolationCount += [int]$policy.violation_count
    }
    Assert-ArtifactNumber $reportPolicyViolationCount $Golden.report_spec.policy_violation_count "report_spec.policy_results violation count"

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "result.result_format_version"
    Assert-ArtifactNumber $result.bytecode_version $Golden.result.bytecode_version "result.bytecode_version"
    Assert-ArtifactValue $result.workflow.kind $Golden.result.workflow_kind "result.workflow.kind"
    Assert-ArtifactNumber @($result.args_schema).Count $Golden.result.args_block_count "result.args_schema count"
    Assert-ArtifactNumber @(@($result.args_schema)[0].fields).Count $Golden.result.args_field_count "result args field count"
    Assert-ArtifactNumber @($result.arg_values).Count $Golden.result.arg_value_count "result.arg_values count"
    $resultArgValue = @($result.arg_values)[0]
    Assert-ArtifactValue $resultArgValue.name $Golden.result.arg_value_name "result.arg_values[0].name"
    Assert-ArtifactValue $resultArgValue.value $Golden.result.arg_value "result.arg_values[0].value"
    Assert-ArtifactValue $resultArgValue.source $Golden.result.arg_value_source "result.arg_values[0].source"
    $resultDataHash = @($result.provenance.data_hashes)[0]
    Assert-ArtifactValue $resultDataHash.source $Golden.result.csv_source_literal "result.provenance.data_hashes[0].source"
    Assert-ArtifactValue $resultDataHash.source_value $Golden.result.csv_source_value "result.provenance.data_hashes[0].source_value"
    Assert-ArtifactNumber $result.object_store.scalar_count $Golden.result.scalar_count "result.object_store.scalar_count"
    Assert-ArtifactNumber $result.object_store.table_count $Golden.result.table_count "result.object_store.table_count"
    Assert-ArtifactNumber $result.object_store.timeseries_count $Golden.result.timeseries_count "result.object_store.timeseries_count"
    Assert-ArtifactNumber $result.provenance.schema_count $Golden.result.schema_count "result.provenance.schema_count"
    Assert-ArtifactNumber $result.provenance.csv_promotion_count $Golden.result.csv_promotion_count "result.provenance.csv_promotion_count"
    Assert-ArtifactNumber @($result.typed_payload.statistics).Count $Golden.result.statistics_count "result.typed_payload.statistics count"
    Assert-ArtifactNumber @($result.typed_payload.integrations).Count $Golden.result.integrations_count "result.typed_payload.integrations count"
    $tableObject = @($result.object_store.objects) | Where-Object { $_.name -eq "sensor" } | Select-Object -First 1
    Assert-Artifact ($null -ne $tableObject) "result.object_store.objects missing sensor table"
    Assert-ArtifactNumber $tableObject.row_count $Golden.result.table_row_count "result.sensor.row_count"
    Assert-ArtifactNumber @($tableObject.columns).Count $Golden.result.table_column_count "result.sensor.columns count"
    Assert-ArtifactNumber @($tableObject.parse_failures).Count $Golden.result.parse_failure_count "result.sensor.parse_failures count"
    $tableConversionFailures = 0
    foreach ($column in @($tableObject.columns)) {
        $tableConversionFailures += @($column.conversion_failures).Count
    }
    Assert-ArtifactNumber $tableConversionFailures $Golden.result.table_conversion_failure_count "result.sensor conversion_failures count"
    $tSupplyColumn = @($tableObject.columns) | Where-Object { $_.name -eq "T_supply" } | Select-Object -First 1
    Assert-Artifact ($null -ne $tSupplyColumn) "result.sensor.columns missing T_supply"
    Assert-ArtifactValue $tSupplyColumn.canonical_unit $Golden.result.t_supply_canonical_unit "result.sensor.T_supply.canonical_unit"
    Assert-ArtifactFloat @($tSupplyColumn.canonical_values)[0] $Golden.result.first_t_supply_k "result.sensor.T_supply.canonical_values[0]"
    $mDotColumn = @($tableObject.columns) | Where-Object { $_.name -eq "m_dot" } | Select-Object -First 1
    Assert-Artifact ($null -ne $mDotColumn) "result.sensor.columns missing m_dot"
    Assert-ArtifactValue $mDotColumn.canonical_unit $Golden.result.m_dot_canonical_unit "result.sensor.m_dot.canonical_unit"
    Assert-ArtifactFloat @($mDotColumn.canonical_values)[0] $Golden.result.first_m_dot_kg_s "result.sensor.m_dot.canonical_values[0]"
    $seriesObject = @($result.object_store.objects) | Where-Object { $_.name -eq "Q_coil" } | Select-Object -First 1
    Assert-Artifact ($null -ne $seriesObject) "result.object_store.objects missing Q_coil TimeSeries"
    Assert-ArtifactNumber $seriesObject.len $Golden.result.timeseries_len "result.Q_coil.len"
    Assert-ArtifactNumber @($seriesObject.points).Count $Golden.result.timeseries_point_count "result.Q_coil.points count"
    Assert-ArtifactFloat @(@($seriesObject.points)[0])[1] $Golden.result.first_timeseries_y_w "result.Q_coil.points[0].y"
    $statsPayload = @($result.typed_payload.statistics)[0]
    Assert-ArtifactValue $statsPayload.status $Golden.result.statistics_status "result.typed_payload.statistics[0].status"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "mean" }).value) $Golden.result.mean_w "result.mean"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "time_weighted_mean" }).value) $Golden.result.time_weighted_mean_w "result.time_weighted_mean"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "max" }).value) $Golden.result.max_w "result.max"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "median" }).value) $Golden.result.median_w "result.median"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "std" }).value) $Golden.result.std_w "result.std"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "p90" }).value) $Golden.result.p90_w "result.p90"
    Assert-ArtifactFloat ((@($statsPayload.statistics) | Where-Object { $_.name -eq "p95" }).value) $Golden.result.p95_w "result.p95"
    $durationAbove = @($statsPayload.statistics) | Where-Object { $_.name -eq "duration_above(5 kW)" } | Select-Object -First 1
    Assert-Artifact ($null -ne $durationAbove) "result.typed_payload.statistics missing duration_above(5 kW)"
    Assert-ArtifactFloat $durationAbove.value $Golden.result.duration_above_5kw_s "result.duration_above"
    Assert-ArtifactValue $durationAbove.unit $Golden.result.duration_above_unit "result.duration_above unit"
    $integrationPayload = @($result.typed_payload.integrations)[0]
    Assert-ArtifactValue $integrationPayload.status $Golden.result.integration_status "result.typed_payload.integrations[0].status"
    Assert-ArtifactFloat $integrationPayload.value $Golden.result.integration_value_j "result.typed_payload.integrations[0].value"
    $resultPolicies = @($result.typed_payload.policy_results)
    Assert-ArtifactNumber $resultPolicies.Count $Golden.result.policy_result_count "result.typed_payload.policy_results count"
    $resultExecutedPolicies = @($resultPolicies | Where-Object { $_.status -eq "executed" })
    $resultValidatedPolicies = @($resultPolicies | Where-Object { $_.status -eq "validated" })
    Assert-ArtifactNumber $resultExecutedPolicies.Count $Golden.result.policy_executed_count "result.typed_payload.policy_results executed count"
    Assert-ArtifactNumber $resultValidatedPolicies.Count $Golden.result.policy_validated_count "result.typed_payload.policy_results validated count"
    $resultPolicyViolationCount = 0
    foreach ($policy in $resultPolicies) {
        $resultPolicyViolationCount += [int]$policy.violation_count
    }
    Assert-ArtifactNumber $resultPolicyViolationCount $Golden.result.policy_violation_count "result.typed_payload.policy_results violation count"

    $plotSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\plots\plot_spec.json")
    Assert-ArtifactValue $plotSpec.format $Golden.plot_spec.format "plot_spec.format"
    Assert-ArtifactNumber $plotSpec.plot_spec_version $Golden.plot_spec.plot_spec_version "plot_spec.plot_spec_version"
    Assert-ArtifactValue $plotSpec.plot_type $Golden.plot_spec.plot_type "plot_spec.plot_type"
    Assert-ArtifactValue $plotSpec.title $Golden.plot_spec.title "plot_spec.title"
    Assert-ArtifactValue $plotSpec.x_axis.unit $Golden.plot_spec.x_unit "plot_spec.x_axis.unit"
    Assert-ArtifactValue $plotSpec.y_axis.unit $Golden.plot_spec.y_unit "plot_spec.y_axis.unit"
    Assert-ArtifactNumber @($plotSpec.series).Count $Golden.plot_spec.series_count "plot_spec.series count"
    $firstSeries = @($plotSpec.series)[0]
    Assert-ArtifactValue $firstSeries.name $Golden.plot_spec.first_series "plot_spec.series[0].name"
    Assert-ArtifactNumber @($firstSeries.points).Count $Golden.plot_spec.point_count "plot_spec.series[0].points count"
    Assert-ArtifactFloat @(@($firstSeries.points)[0])[0] $Golden.plot_spec.first_point_x "plot_spec.series[0].points[0].x"
    Assert-ArtifactFloat @(@($firstSeries.points)[0])[1] $Golden.plot_spec.first_point_y "plot_spec.series[0].points[0].y"
    Assert-ArtifactFloat @(@($firstSeries.points)[3])[0] $Golden.plot_spec.last_point_x "plot_spec.series[0].points[3].x"
    Assert-ArtifactFloat @(@($firstSeries.points)[3])[1] $Golden.plot_spec.last_point_y "plot_spec.series[0].points[3].y"

    Remove-Item -LiteralPath (Join-Path $RepoRoot "dist\main-standalone") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "build" $Golden.source "--standalone" "--profile" "repro"
    $engpkg = Read-KeyValueArtifact (Join-Path $RepoRoot "dist\main-standalone\main.engpkg")
    Assert-ArtifactValue $engpkg["format"] $Golden.engpkg.format "engpkg.format"
    Assert-ArtifactValue $engpkg["package_format_version"] $Golden.engpkg.package_format_version "engpkg.package_format_version"
    Assert-ArtifactValue $engpkg["runtime_abi"] $Golden.engpkg.runtime_abi "engpkg.runtime_abi"
    Assert-ArtifactValue $engpkg["profile"] $Golden.engpkg.profile "engpkg.profile"
    Assert-ArtifactValue $engpkg["runner"] $Golden.engpkg.runner "engpkg.runner"
    Assert-ArtifactValue $engpkg["engine"] $Golden.engpkg.engine "engpkg.engine"
    Assert-ArtifactValue $engpkg["source_root"] $Golden.engpkg.source_root "engpkg.source_root"
    Assert-ArtifactValue $engpkg["artifact_root"] $Golden.engpkg.artifact_root "engpkg.artifact_root"
    Assert-ArtifactValue $engpkg["source"] $Golden.engpkg.source "engpkg.source"
    Assert-ArtifactValue $engpkg["bytecode"] $Golden.engpkg.bytecode "engpkg.bytecode"
    Assert-ArtifactValue $engpkg["workflow"] $Golden.engpkg.workflow "engpkg.workflow"
    Assert-ArtifactValue $engpkg["args_schema"] $Golden.engpkg.args_schema "engpkg.args_schema"
    Assert-ArtifactValue $engpkg["args_field_count"] $Golden.engpkg.args_field_count "engpkg.args_field_count"
    Assert-ArtifactValue $engpkg["args_help"] $Golden.engpkg.args_help "engpkg.args_help"
    Assert-ArtifactValue $engpkg["dependency_count"] $Golden.engpkg.dependency_count "engpkg.dependency_count"
    Assert-ArtifactValue $engpkg["dependencies"] $Golden.engpkg.dependencies "engpkg.dependencies"
    Assert-Artifact ($engpkg["dependency_hashes"].Contains(($Golden.engpkg.dependencies + ":"))) "engpkg.dependency_hashes does not include dependency path"
    $argsHelpPath = Join-Path $RepoRoot "dist\main-standalone\ARGS_HELP.txt"
    Assert-Artifact (Test-Path -LiteralPath $argsHelpPath -PathType Leaf) "missing standalone ARGS_HELP.txt"
    $argsHelpText = Get-Content -LiteralPath $argsHelpPath -Raw -Encoding UTF8
    Assert-Artifact ($argsHelpText.Contains("Args metadata")) "standalone ARGS_HELP.txt does not mention Args metadata"
    Assert-Artifact ($argsHelpText.Contains("--input <CsvFile>")) "standalone ARGS_HELP.txt does not mention --input"

    $lock = Read-KeyValueArtifact (Join-Path $RepoRoot "dist\main-standalone\main.lock")
    Assert-ArtifactValue $lock["package_format_version"] "1" "lock.package_format_version"
    Assert-ArtifactValue $lock["runtime_abi"] "eng-runtime-cli-v1" "lock.runtime_abi"
    Assert-ArtifactValue $lock["bytecode_version"] "1" "lock.bytecode_version"
    Assert-ArtifactValue $lock["result_format_version"] "1" "lock.result_format_version"
    Assert-ArtifactValue $lock["report_schema_version"] "1" "lock.report_schema_version"
    Assert-ArtifactValue $lock["plot_spec_version"] "1" "lock.plot_spec_version"
    Assert-ArtifactValue $lock["profile"] "repro" "lock.profile"
    Assert-ArtifactValue $lock["workflow"] $Golden.engpkg.workflow "lock.workflow"
    Assert-ArtifactValue $lock["dependency_count"] $Golden.engpkg.dependency_count "lock.dependency_count"
    Assert-Artifact ($lock["dependency_hashes"].Contains(($Golden.engpkg.dependencies + ":"))) "lock.dependency_hashes does not include dependency path"
}

function Assert-SystemGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--save-artifacts"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "system review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "system review.review_schema_version"
    Assert-ArtifactNumber $review.syntax_summary.scripts $Golden.review.scripts "system review.syntax_summary.scripts"
    Assert-ArtifactNumber $review.syntax_summary.schemas $Golden.review.schemas "system review.syntax_summary.schemas"
    Assert-ArtifactNumber $review.syntax_summary.structs $Golden.review.structs "system review.syntax_summary.structs"
    Assert-ArtifactNumber $review.syntax_summary.args_fields $Golden.review.args_fields "system review.syntax_summary.args_fields"
    Assert-ArtifactNumber $review.syntax_summary.systems $Golden.review.systems "system review.syntax_summary.systems"
    Assert-ArtifactNumber $review.syntax_summary.equations $Golden.review.equations "system review.syntax_summary.equations"
    Assert-ArtifactNumber @($review.args_summary).Count $Golden.review.args_block_count "system review.args_summary count"
    Assert-ArtifactNumber @(@($review.args_summary)[0].fields).Count $Golden.review.args_field_count "system review args field count"
    Assert-ArtifactNumber @($review.arg_values).Count $Golden.review.arg_value_count "system review.arg_values count"
    $reviewArgValue = @($review.arg_values)[0]
    Assert-ArtifactValue $reviewArgValue.name $Golden.review.arg_value_name "system review.arg_values[0].name"
    Assert-ArtifactValue $reviewArgValue.value $Golden.review.arg_value "system review.arg_values[0].value"
    Assert-ArtifactValue $reviewArgValue.source $Golden.review.arg_value_source "system review.arg_values[0].source"
    Assert-ArtifactNumber @($review.system_summary).Count $Golden.review.system_summary_count "system review.system_summary count"
    Assert-ArtifactNumber @(@($review.system_summary)[0].residuals).Count $Golden.review.residual_count "system review residual count"
    Assert-ArtifactNumber @($review.system_ir).Count $Golden.review.system_ir_count "system review.system_ir count"
    $reviewSystemIr = @($review.system_ir)[0]
    Assert-ArtifactValue $reviewSystemIr.solver_boundary.status $Golden.review.solver_status "system review.solver_boundary.status"
    Assert-ArtifactValue $reviewSystemIr.solver_plan.status $Golden.review.solver_plan_status "system review.solver_plan.status"
    Assert-ArtifactNumber @($reviewSystemIr.solver_plan.solve_order).Count $Golden.review.solve_order_count "system review solver_plan.solve_order count"
    Assert-ArtifactValue $reviewSystemIr.solver_plan.ode_runner.status $Golden.review.ode_runner_status "system review solver_plan.ode_runner.status"
    Assert-ArtifactNumber @($reviewSystemIr.solver_plan.jacobian_seed).Count $Golden.review.jacobian_seed_count "system review solver_plan.jacobian_seed count"
    Assert-ArtifactNumber @(@($reviewSystemIr.equations)[0].dependencies).Count $Golden.review.dependency_count "system review IR dependency count"
    Assert-ArtifactNumber @(@($reviewSystemIr.equations)[0].derivative_states).Count $Golden.review.derivative_state_count "system review IR derivative state count"

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "system report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "system report_spec.report_schema_version"
    Assert-ArtifactNumber $reportSpec.provenance.schema_count $Golden.report_spec.schema_count "system report_spec.provenance.schema_count"
    Assert-ArtifactNumber $reportSpec.provenance.csv_promotion_count $Golden.report_spec.csv_promotion_count "system report_spec.provenance.csv_promotion_count"
    Assert-ArtifactNumber @($reportSpec.args_summary).Count $Golden.report_spec.args_block_count "system report_spec.args_summary count"
    Assert-ArtifactNumber @(@($reportSpec.args_summary)[0].fields).Count $Golden.report_spec.args_field_count "system report_spec args field count"
    Assert-ArtifactNumber @($reportSpec.arg_values).Count $Golden.report_spec.arg_value_count "system report_spec.arg_values count"
    $reportArgValue = @($reportSpec.arg_values)[0]
    Assert-ArtifactValue $reportArgValue.name $Golden.report_spec.arg_value_name "system report_spec.arg_values[0].name"
    Assert-ArtifactValue $reportArgValue.value $Golden.report_spec.arg_value "system report_spec.arg_values[0].value"
    Assert-ArtifactValue $reportArgValue.source $Golden.report_spec.arg_value_source "system report_spec.arg_values[0].source"
    Assert-ArtifactNumber $reportSpec.provenance.system_count $Golden.report_spec.system_count "system report_spec.provenance.system_count"
    Assert-ArtifactNumber $reportSpec.provenance.equation_count $Golden.report_spec.equation_count "system report_spec.provenance.equation_count"
    Assert-ArtifactNumber $reportSpec.provenance.residual_count $Golden.report_spec.residual_count "system report_spec.provenance.residual_count"
    Assert-ArtifactNumber @($reportSpec.system_ir).Count $Golden.report_spec.system_ir_count "system report_spec.system_ir count"
    $reportSystemIr = @($reportSpec.system_ir)[0]
    Assert-ArtifactValue $reportSystemIr.solver_boundary.status $Golden.report_spec.solver_status "system report_spec.solver_boundary.status"
    Assert-ArtifactValue $reportSystemIr.solver_plan.status $Golden.report_spec.solver_plan_status "system report_spec.solver_plan.status"
    Assert-ArtifactValue $reportSystemIr.solver_plan.method $Golden.report_spec.solver_method "system report_spec.solver_plan.method"
    Assert-ArtifactNumber @($reportSystemIr.solver_plan.solve_order).Count $Golden.report_spec.solve_order_count "system report_spec solver_plan.solve_order count"
    Assert-ArtifactValue $reportSystemIr.solver_plan.ode_runner.status $Golden.report_spec.ode_runner_status "system report_spec solver_plan.ode_runner.status"
    Assert-ArtifactNumber @($reportSystemIr.solver_plan.jacobian_seed).Count $Golden.report_spec.jacobian_seed_count "system report_spec solver_plan.jacobian_seed count"
    Assert-ArtifactNumber @(@($reportSystemIr.equations)[0].dependencies).Count $Golden.report_spec.dependency_count "system report_spec IR dependency count"
    Assert-ArtifactNumber @(@($reportSystemIr.equations)[0].derivative_states).Count $Golden.report_spec.derivative_state_count "system report_spec IR derivative state count"

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "system result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "system result.result_format_version"
    Assert-ArtifactNumber $result.bytecode_version $Golden.result.bytecode_version "system result.bytecode_version"
    Assert-ArtifactValue $result.workflow.kind $Golden.result.workflow_kind "system result.workflow.kind"
    Assert-ArtifactNumber @($result.args_schema).Count $Golden.result.args_block_count "system result.args_schema count"
    Assert-ArtifactNumber @(@($result.args_schema)[0].fields).Count $Golden.result.args_field_count "system result args field count"
    Assert-ArtifactNumber @($result.arg_values).Count $Golden.result.arg_value_count "system result.arg_values count"
    $resultArgValue = @($result.arg_values)[0]
    Assert-ArtifactValue $resultArgValue.name $Golden.result.arg_value_name "system result.arg_values[0].name"
    Assert-ArtifactValue $resultArgValue.value $Golden.result.arg_value "system result.arg_values[0].value"
    Assert-ArtifactValue $resultArgValue.source $Golden.result.arg_value_source "system result.arg_values[0].source"
    Assert-ArtifactNumber $result.object_store.table_count $Golden.result.table_count "system result.object_store.table_count"
    Assert-ArtifactNumber $result.object_store.timeseries_count $Golden.result.timeseries_count "system result.object_store.timeseries_count"
    Assert-ArtifactNumber $result.provenance.system_count $Golden.result.system_count "system result.provenance.system_count"
    Assert-ArtifactNumber $result.provenance.equation_count $Golden.result.equation_count "system result.provenance.equation_count"
    Assert-ArtifactNumber $result.provenance.residual_count $Golden.result.residual_count "system result.provenance.residual_count"
    Assert-ArtifactNumber @($result.typed_payload.solver_boundaries).Count $Golden.result.solver_boundary_count "system result.typed_payload.solver_boundaries count"
    Assert-ArtifactNumber @($result.typed_payload.system_ir).Count $Golden.result.system_ir_count "system result.typed_payload.system_ir count"
    $resultSolverBoundary = @($result.typed_payload.solver_boundaries)[0]
    Assert-ArtifactValue $resultSolverBoundary.status $Golden.result.solver_status "system result.solver_boundary.status"
    $resultSystemIr = @($result.typed_payload.system_ir)[0]
    Assert-ArtifactValue $resultSystemIr.solver_plan.status $Golden.result.solver_plan_status "system result.solver_plan.status"
    Assert-ArtifactValue $resultSystemIr.solver_plan.method $Golden.result.solver_method "system result.solver_plan.method"
    Assert-ArtifactNumber @($resultSystemIr.solver_plan.solve_order).Count $Golden.result.solve_order_count "system result solver_plan.solve_order count"
    Assert-ArtifactValue $resultSystemIr.solver_plan.ode_runner.status $Golden.result.ode_runner_status "system result solver_plan.ode_runner.status"
    Assert-ArtifactNumber @($resultSystemIr.solver_plan.jacobian_seed).Count $Golden.result.jacobian_seed_count "system result solver_plan.jacobian_seed count"
    Assert-ArtifactNumber @(@($resultSystemIr.equations)[0].dependencies).Count $Golden.result.dependency_count "system result IR dependency count"
    Assert-ArtifactNumber @(@($resultSystemIr.equations)[0].derivative_states).Count $Golden.result.derivative_state_count "system result IR derivative state count"
    $resultSystemPayload = @($result.typed_payload.systems)[0]
    Assert-ArtifactValue $resultSystemPayload.solver_result.status $Golden.result.solver_result_status "system result.solver_result.status"
    Assert-ArtifactNumber $resultSystemPayload.solver_result.step_count $Golden.result.solver_step_count "system result.solver_result.step_count"
    Assert-ArtifactFloat $resultSystemPayload.solver_result.duration $Golden.result.solver_duration_s "system result.solver_result.duration"
    Assert-ArtifactFloat $resultSystemPayload.solver_result.time_step $Golden.result.solver_time_step_s "system result.solver_result.time_step"
    Assert-ArtifactFloat $resultSystemPayload.solver_result.final_value $Golden.result.solver_final_temp_deg_c "system result.solver_result.final_value"
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
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "run" $example "--save-artifacts"
}

function Invoke-IdeCheck {
    $ExtensionRoot = Join-Path $RepoRoot "tools\vscode-englang"
    $PackageJsonPath = Join-Path $ExtensionRoot "package.json"
    $ExtensionJsPath = Join-Path $ExtensionRoot "extension.js"

    if (-not (Test-Path $PackageJsonPath)) {
        throw "missing VS Code extension package.json at $PackageJsonPath"
    }
    if (-not (Test-Path $ExtensionJsPath)) {
        throw "missing VS Code extension entrypoint at $ExtensionJsPath"
    }

    $Package = Get-Content -LiteralPath $PackageJsonPath -Raw | ConvertFrom-Json
    if ($Package.name -ne "englang") {
        throw "VS Code extension package name must be englang"
    }
    if ($Package.main -ne "./extension.js") {
        throw "VS Code extension main must be ./extension.js"
    }
    $Language = $Package.contributes.languages | Select-Object -First 1
    if ($Language.id -ne "englang") {
        throw "VS Code extension must contribute englang language id"
    }
    if ($Language.extensions -notcontains ".eng") {
        throw "VS Code extension must register .eng files"
    }
    $Commands = @($Package.contributes.commands | ForEach-Object { $_.command })
    foreach ($Required in @("englang.checkFile", "englang.runFile", "englang.openReport")) {
        if ($Commands -notcontains $Required) {
            throw "VS Code extension missing command $Required"
        }
    }
    $Properties = $Package.contributes.configuration.properties
    foreach ($RequiredProperty in @("englang.runtimePath", "englang.lspPath", "englang.diagnosticsBackend", "englang.lintOnSave", "englang.runEntry")) {
        if ($null -eq $Properties.$RequiredProperty) {
            throw "VS Code extension missing configuration property $RequiredProperty"
        }
    }
    $BackendEnum = @($Properties."englang.diagnosticsBackend".enum)
    foreach ($RequiredBackend in @("eng-cli", "lsp-snapshot")) {
        if ($BackendEnum -notcontains $RequiredBackend) {
            throw "VS Code extension diagnosticsBackend missing enum value $RequiredBackend"
        }
    }

    $Node = Get-Command node -ErrorAction SilentlyContinue
    if ($null -ne $Node) {
        Invoke-Native $Node.Source "--check" $ExtensionJsPath
    } else {
        Write-Host "Node not found; skipped extension.js syntax check."
    }

    Write-Host "IDE extension check passed."
}

function Invoke-LspCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "test" "-p" "eng_lsp" "--test" "stdio" "--" "--nocapture"
    Invoke-Native $cargo "run" "-p" "eng_lsp" "--" "--smoke"
    Invoke-Native $cargo "run" "-p" "eng_lsp" "--" "--snapshot-check" "examples\official\01_csv_plot\main.eng"
    Write-Host "LSP check passed."
}

function Invoke-JitCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "test" "-p" "eng_jit" "--" "--nocapture"
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "jit-plan" "examples\official\01_csv_plot\main.eng"
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "jit-plan" "examples\official\01_csv_plot\main.eng" "--backend" "native-preview"
    Invoke-Native $cargo "run" "-p" "eng_cli" "--" "jit-bench" "examples\official\01_csv_plot\main.eng" "--iterations" "1"
    Write-Host "JIT plan check passed."
}

function Invoke-Ide {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "run" "-p" "eng_ide" "--" @Rest
}

function Invoke-DevCurrent {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "--release" "-p" "eng_cli" "-p" "eng_ide" "-p" "eng_lsp"

    $CurrentRoot = Join-Path $RepoRoot "dist\dev-current"
    Remove-Item -LiteralPath $CurrentRoot -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $CurrentRoot | Out-Null

    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng.exe") (Join-Path $CurrentRoot "eng.exe")
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng-ide.exe") (Join-Path $CurrentRoot "eng-ide.exe")
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng-lsp.exe") (Join-Path $CurrentRoot "eng-lsp.exe")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "examples") (Join-Path $CurrentRoot "examples")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "stdlib") (Join-Path $CurrentRoot "stdlib")
    $CurrentDocsRoot = Join-Path $CurrentRoot "docs"
    New-Item -ItemType Directory -Force -Path $CurrentDocsRoot | Out-Null
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "docs\tutorials") (Join-Path $CurrentDocsRoot "tutorials")
    $CurrentGrammarGuidePath = Join-Path $CurrentDocsRoot "EngLang_Language_Grammar_Guide.pdf"
    if (-not (New-GrammarGuideWithOodocs -Path $CurrentGrammarGuidePath -Version (Get-PublicVersion))) {
        throw "Could not generate EngLang language grammar guide for dev-current."
    }

    $Version = Get-WorkspaceVersion
    $GitCommit = try {
        (& git rev-parse --short HEAD 2>$null)
    } catch {
        "unknown"
    }
    $EngHash = (Get-FileHash -Algorithm SHA256 (Join-Path $CurrentRoot "eng.exe")).Hash.ToLowerInvariant()
    $IdeHash = (Get-FileHash -Algorithm SHA256 (Join-Path $CurrentRoot "eng-ide.exe")).Hash.ToLowerInvariant()
    $LspHash = (Get-FileHash -Algorithm SHA256 (Join-Path $CurrentRoot "eng-lsp.exe")).Hash.ToLowerInvariant()

    Set-Content -Path (Join-Path $CurrentRoot "README.txt") -Encoding ascii -Value @"
EngLang dev-current

This folder is the current commit's non-release test display build.
Run eng-ide.exe from this folder to open the native IDE with bundled examples,
stdlib files, tutorials, and the language grammar guide.

Docs:
  docs\EngLang_Language_Grammar_Guide.pdf

Smoke commands:
  eng-ide.exe --smoke
  eng-lsp.exe --smoke
  eng.exe doctor
  eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts

Regenerate:
  .\dev.bat dev-current

Clean transient generated files while preserving this folder:
  .\dev.bat clean-generated
"@

    Set-Content -Path (Join-Path $CurrentRoot "MANIFEST.txt") -Encoding ascii -Value @"
EngLang dev-current manifest

version = $Version
commit = $GitCommit
generated_at_local = $(Get-Date -Format "yyyy-MM-dd HH:mm:ss zzz")
eng_sha256 = $EngHash
eng_ide_sha256 = $IdeHash
eng_lsp_sha256 = $LspHash
"@

    Write-Host "Dev current build prepared."
    Write-Host "IDE: $(Join-Path $CurrentRoot "eng-ide.exe")"
    Write-Host "Manifest: $(Join-Path $CurrentRoot "MANIFEST.txt")"
}

function New-VsixManifest {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path,

        [Parameter(Mandatory = $true)]
        [string] $Version
    )

    Set-Content -Path $Path -Encoding ascii -Value @"
<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="englang" Version="$Version" Publisher="englang" />
    <DisplayName>EngLang</DisplayName>
    <Description xml:space="preserve">EngLang IDE preview with diagnostics, hover, completion, and run commands.</Description>
    <Tags>EngLang, language, engineering</Tags>
    <Categories>Programming Languages</Categories>
    <GalleryFlags>Public</GalleryFlags>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="^1.85.0" />
    </Properties>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code" />
  </Installation>
  <Dependencies />
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Code.Content" Path="extension" Addressable="true" />
  </Assets>
</PackageManifest>
"@
}

function Invoke-IdePackage {
    param(
        [Parameter(Mandatory = $true)]
        [string] $PackageRoot
    )

    Invoke-IdeCheck
    $Version = Get-WorkspaceVersion
    $ExtensionSource = Join-Path $RepoRoot "tools\vscode-englang"
    $ToolsRoot = Join-Path $PackageRoot "tools"
    $ExtensionOut = Join-Path $ToolsRoot "vscode-englang"
    $VsixStage = Join-Path $RepoRoot "build\vscode-vsix"
    $VsixExtensionRoot = Join-Path $VsixStage "extension"
    $VsixPath = Join-Path $ToolsRoot "englang-vscode-preview-$Version.vsix"
    $ReleaseEng = Join-Path $RepoRoot "target\release\eng.exe"
    $ReleaseLsp = Join-Path $RepoRoot "target\release\eng-lsp.exe"

    New-Item -ItemType Directory -Force -Path $ToolsRoot | Out-Null
    Remove-Item -LiteralPath $ExtensionOut -Recurse -Force -ErrorAction SilentlyContinue
    Copy-Item -Recurse -Force $ExtensionSource $ExtensionOut

    Remove-Item -LiteralPath $VsixStage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $VsixExtensionRoot | Out-Null
    Copy-Item -Recurse -Force (Join-Path $ExtensionSource "*") $VsixExtensionRoot
    New-Item -ItemType Directory -Force -Path (Join-Path $VsixExtensionRoot "bin") | Out-Null
    Copy-Item -Force $ReleaseEng (Join-Path $VsixExtensionRoot "bin\eng.exe")
    Copy-Item -Force $ReleaseLsp (Join-Path $VsixExtensionRoot "bin\eng-lsp.exe")
    New-VsixManifest -Path (Join-Path $VsixStage "extension.vsixmanifest") -Version $Version
    $VsixZipPath = "$VsixPath.zip"
    Remove-Item -LiteralPath $VsixZipPath -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path (Join-Path $VsixStage "*") -DestinationPath $VsixZipPath -Force
    Move-Item -LiteralPath $VsixZipPath -Destination $VsixPath -Force

    Write-Host "VS Code extension prepared at $ExtensionOut"
    Write-Host "VSIX prepared at $VsixPath"
}

function Escape-PdfText {
    param([Parameter(Mandatory = $true)][string] $Text)
    return $Text.Replace('\', '\\').Replace('(', '\(').Replace(')', '\)')
}

function Split-PdfText {
    param(
        [Parameter(Mandatory = $true)][string] $Text,
        [Parameter(Mandatory = $true)][int] $MaxChars
    )

    $words = $Text.Split(" ", [System.StringSplitOptions]::RemoveEmptyEntries)
    $lines = New-Object System.Collections.Generic.List[string]
    $current = ""
    foreach ($word in $words) {
        if ($current.Length -eq 0) {
            $current = $word
        } elseif (($current.Length + 1 + $word.Length) -le $MaxChars) {
            $current = "$current $word"
        } else {
            $lines.Add($current) | Out-Null
            $current = $word
        }
    }
    if ($current.Length -gt 0) {
        $lines.Add($current) | Out-Null
    }
    return $lines
}

function New-UserGuideWithOodocs {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $Version
    )

    $python = Get-PortablePython
    $script = Join-Path $RepoRoot "docs\user\build_user_docs.py"
    if ($null -eq $python -or -not (Test-Path $script)) {
        return $false
    }

    try {
        Invoke-Native $python $script "--pdf" $Path "--version" $Version
        return (Test-Path $Path)
    } catch {
        Write-Host "OODocs user guide generation failed; using fallback PDF generator."
        Write-Host $_
        return $false
    }
}

function New-GrammarGuideWithOodocs {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $Version
    )

    $python = Get-PortablePython
    $script = Join-Path $RepoRoot "docs\user\build_language_grammar_docs.py"
    if ($null -eq $python -or -not (Test-Path $script)) {
        return $false
    }

    try {
        Invoke-Native $python $script "--pdf" $Path "--version" $Version
        return (Test-Path $Path)
    } catch {
        Write-Host "OODocs grammar guide generation failed."
        Write-Host $_
        return $false
    }
}

function Invoke-GrammarDocs {
    Set-DevEnvironment
    $Version = Get-PublicVersion
    $Output = Join-Path $RepoRoot "build\docs\EngLang_Language_Grammar_Guide.pdf"
    if (-not (New-GrammarGuideWithOodocs -Path $Output -Version $Version)) {
        throw "grammar docs PDF was not created: $Output"
    }
    Write-Host "Grammar docs generated at $Output"
}

function New-UserGuidePdf {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $Version
    )

    $sections = @(
        @{ Kind = "title"; Text = "EngLang User Test Guide" },
        @{ Kind = "subtitle"; Text = "Portable Windows package v$Version" },
        @{ Kind = "body"; Text = "EngLang is a native engineering language for workflows where units, physical quantities, schemas, axes, statistics, plots, reports, and provenance are checked as part of the program. This PDF is the curated user-facing guide for the portable package; developer notes and master plans stay in the repository." },
        @{ Kind = "h1"; Text = "1. Package Contents" },
        @{ Kind = "body"; Text = "The portable folder contains eng.exe for command-line execution, eng-ide.exe for native GUI testing, eng-lsp.exe for experimental editor-service smoke checks, official examples, stdlib language seeds, tools for the optional VS Code extension preview, and this PDF. It intentionally does not ship the full developer documentation tree." },
        @{ Kind = "h1"; Text = "2. First Smoke Test" },
        @{ Kind = "step"; Text = "Open a command prompt in the extracted folder." },
        @{ Kind = "step"; Text = "Run: eng.exe doctor" },
        @{ Kind = "step"; Text = "Run: eng-ide.exe --smoke" },
        @{ Kind = "step"; Text = "Run: eng-lsp.exe --smoke" },
        @{ Kind = "body"; Text = "All three commands should exit successfully. The doctor command verifies runtime, standard library, unit registry, plot renderer, report generator, write permission, and example files. The IDE smoke command verifies that examples and compiler completion metadata are discoverable. The LSP smoke command verifies the experimental editor-service diagnostics, completion, and hover metadata path." },
        @{ Kind = "h1"; Text = "3. Native IDE Workflow" },
        @{ Kind = "step"; Text = "Run: eng-ide.exe" },
        @{ Kind = "step"; Text = "Use Explorer to open examples/official/03_integrated_hvac/main.eng or create a scratch .eng file." },
        @{ Kind = "step"; Text = "Use Check for lint diagnostics. Error and warning counts are visible in the toolbar and details are listed in Problems." },
        @{ Kind = "step"; Text = "Use Ctrl+Space in the editor to update completion filtering, then insert keywords, quantity kinds, units, or snippets from Completions." },
        @{ Kind = "step"; Text = "Use Run to generate result artifacts. The IDE previews PlotSpec data and exposes report, plot, result, review, and manifest paths." },
        @{ Kind = "body"; Text = "The IDE uses the same compiler and runtime crates as eng.exe. Diagnostics, symbols, completions, run artifacts, and report generation therefore test the real core path rather than duplicated editor logic." },
        @{ Kind = "h1"; Text = "4. Integrated HVAC Example" },
        @{ Kind = "body"; Text = "The integrated HVAC example is the recommended user test because one file exercises typed CSV promotion, DateTime parsing, missing-value interpolation, schema constraints, HeatRate calculation, TimeSeries statistics, trapezoidal integration, PlotSpec/SVG/report output, and the simple thermal system fixed-step ODE preview." },
        @{ Kind = "body"; Text = "From the command line, run: eng.exe run examples/official/03_integrated_hvac/main.eng --save-artifacts" },
        @{ Kind = "h1"; Text = "5. Expected Output" },
        @{ Kind = "body"; Text = "After a successful run, inspect build/result/report.html first. The result folder also contains result.engres, review.json, report_spec.json, plots/plot_spec.json, plots/plot_manifest.json, and plots/timeseries.svg." },
        @{ Kind = "body"; Text = "The result should record policy_results with interpolation executed, statistics including median/std/p90/p95/duration_above, an integration result for E_coil, and systems[0].solver_result.status = computed." },
        @{ Kind = "h1"; Text = "6. Useful User Edits" },
        @{ Kind = "step"; Text = "Change the plot title and run again to verify report regeneration." },
        @{ Kind = "step"; Text = "Change duration_above(5 kW) to duration_above(4.5 kW) and compare computed statistics." },
        @{ Kind = "step"; Text = "Temporarily change m_dot <= 0.30 kg/s to m_dot <= 0.20 kg/s and inspect policy results." },
        @{ Kind = "step"; Text = "Type Heat and use completion to insert HeatRate or HeatCapacity." },
        @{ Kind = "h1"; Text = "7. Troubleshooting" },
        @{ Kind = "body"; Text = "If a run fails, check Problems first, then run eng.exe check <file.eng> from the same folder. If the plot preview is empty, open the Artifacts tab and check plots/plot_spec.json and plots/timeseries.svg." },
        @{ Kind = "body"; Text = "If a CSV path fails, keep relative paths anchored next to the source file, as in the official examples. If a report does not open, open build/result/report.html manually." },
        @{ Kind = "h1"; Text = "8. Current Boundaries" },
        @{ Kind = "body"; Text = "This is a public preview release. The supported user-test workflows cover CSV promote, unit-aware TimeSeries calculations, PlotSpec/SVG output, review/report artifacts, basic packaged execution, and the native tester IDE. Language and artifact formats are not yet stable. Uncertainty, ML, LSP, JIT/AOT, and domain/component work are future tracks unless explicitly marked preview-supported." }
    )

    $pages = New-Object System.Collections.Generic.List[string]
    $content = New-Object System.Collections.Generic.List[string]
    $script:EngPdfY = 740
    $script:EngPdfPageNumber = 1

    function Add-PdfPage {
        if ($content.Count -gt 0) {
            $content.Add("BT /F1 8 Tf 54 34 Td (EngLang v$Version user test guide - page $script:EngPdfPageNumber) Tj ET") | Out-Null
            $pages.Add(($content -join "`n")) | Out-Null
            $content.Clear()
            $script:EngPdfY = 740
            $script:EngPdfPageNumber += 1
        }
    }

    function Add-PdfLine {
        param(
            [string] $Text,
            [int] $Size,
            [int] $Leading,
            [string] $Font = "F1",
            [int] $X = 54
        )
        if ($script:EngPdfY -lt 72) {
            Add-PdfPage
        }
        $escaped = Escape-PdfText $Text
        $content.Add("BT /$Font $Size Tf $X $script:EngPdfY Td ($escaped) Tj ET") | Out-Null
        $script:EngPdfY -= $Leading
    }

    foreach ($section in $sections) {
        switch ($section.Kind) {
            "title" {
                Add-PdfLine $section.Text 22 30 "F2"
            }
            "subtitle" {
                Add-PdfLine $section.Text 12 28 "F1"
            }
            "h1" {
                $script:EngPdfY -= 8
                Add-PdfLine $section.Text 15 22 "F2"
            }
            "step" {
                $stepLines = @(Split-PdfText $section.Text 84)
                for ($lineIndex = 0; $lineIndex -lt $stepLines.Count; $lineIndex++) {
                    if ($lineIndex -eq 0) {
                        Add-PdfLine "- $($stepLines[$lineIndex])" 10 15 "F1" 66
                    } else {
                        Add-PdfLine "  $($stepLines[$lineIndex])" 10 15 "F1" 78
                    }
                }
                $script:EngPdfY -= 3
            }
            default {
                foreach ($line in (Split-PdfText $section.Text 92)) {
                    Add-PdfLine $line 10 15 "F1"
                }
                $script:EngPdfY -= 6
            }
        }
    }
    Add-PdfPage

    $objects = New-Object System.Collections.Generic.List[string]
    $objects.Add("<< /Type /Catalog /Pages 2 0 R >>") | Out-Null
    $pageKids = New-Object System.Collections.Generic.List[string]
    $pageObjectStart = 5
    $contentObjectStart = $pageObjectStart + $pages.Count
    for ($index = 0; $index -lt $pages.Count; $index++) {
        $pageKids.Add("$($pageObjectStart + $index) 0 R") | Out-Null
    }
    $objects.Add("<< /Type /Pages /Kids [$($pageKids -join ' ')] /Count $($pages.Count) >>") | Out-Null
    $objects.Add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>") | Out-Null
    $objects.Add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>") | Out-Null

    for ($index = 0; $index -lt $pages.Count; $index++) {
        $contentObject = $contentObjectStart + $index
        $objects.Add("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents $contentObject 0 R >>") | Out-Null
    }
    foreach ($page in $pages) {
        $bytes = [System.Text.Encoding]::ASCII.GetBytes($page)
        $objects.Add("<< /Length $($bytes.Length) >>`nstream`n$page`nendstream") | Out-Null
    }

    $pdf = New-Object System.Text.StringBuilder
    [void] $pdf.Append("%PDF-1.4`n")
    $offsets = New-Object System.Collections.Generic.List[int]
    for ($index = 0; $index -lt $objects.Count; $index++) {
        $offsets.Add([System.Text.Encoding]::ASCII.GetByteCount($pdf.ToString())) | Out-Null
        [void] $pdf.Append("$($index + 1) 0 obj`n$($objects[$index])`nendobj`n")
    }
    $xrefOffset = [System.Text.Encoding]::ASCII.GetByteCount($pdf.ToString())
    [void] $pdf.Append("xref`n0 $($objects.Count + 1)`n")
    [void] $pdf.Append("0000000000 65535 f `n")
    foreach ($offset in $offsets) {
        [void] $pdf.Append(("{0:D10} 00000 n `n" -f $offset))
    }
    [void] $pdf.Append("trailer`n<< /Size $($objects.Count + 1) /Root 1 0 R >>`nstartxref`n$xrefOffset`n%%EOF`n")

    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    [System.IO.File]::WriteAllBytes($Path, [System.Text.Encoding]::ASCII.GetBytes($pdf.ToString()))
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
    $PublicVersion = Get-PublicVersion
    $PackageRoot = Join-Path $RepoRoot "dist\englang-preview"
    $ZipPath = Join-Path $RepoRoot "dist\englang-preview-v$PublicVersion-windows-x64.zip"
    $ChecksumPath = "$ZipPath.sha256"
    $ReleaseGuidePath = Join-Path $RepoRoot "dist\englang-user-test-guide-v$PublicVersion.pdf"
    Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ZipPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ReleaseGuidePath -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng.exe") (Join-Path $PackageRoot "eng.exe")
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng-ide.exe") (Join-Path $PackageRoot "eng-ide.exe")
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng-lsp.exe") (Join-Path $PackageRoot "eng-lsp.exe")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "examples") (Join-Path $PackageRoot "examples")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "stdlib") (Join-Path $PackageRoot "stdlib")
    New-Item -ItemType Directory -Force -Path (Join-Path $PackageRoot "docs") | Out-Null
    $PackageGuidePath = Join-Path $PackageRoot "docs\EngLang_User_Test_Guide.pdf"
    if (-not (New-UserGuideWithOodocs -Path $PackageGuidePath -Version $PublicVersion)) {
        New-UserGuidePdf -Path $PackageGuidePath -Version $PublicVersion
    }
    $PackageGrammarGuidePath = Join-Path $PackageRoot "docs\EngLang_Language_Grammar_Guide.pdf"
    if (-not (New-GrammarGuideWithOodocs -Path $PackageGrammarGuidePath -Version $PublicVersion)) {
        throw "Could not generate EngLang language grammar guide with OODocs."
    }
    Copy-Item -Force $PackageGuidePath $ReleaseGuidePath
    Invoke-IdePackage -PackageRoot $PackageRoot
    Set-Content -Path (Join-Path $PackageRoot "README.txt") -Encoding ascii -Value @"
EngLang portable package

This folder is self-contained for preview execution. Rust and Python are not
required on the target PC.

Recommended smoke commands:
  eng.exe doctor
  eng-ide.exe --smoke
  eng-lsp.exe --smoke
  eng-ide.exe
  eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts
  eng.exe run examples\official\02_simple_system\main.eng --save-artifacts
  eng.exe run examples\official\03_integrated_hvac\main.eng --save-artifacts
  eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
  dist\main-standalone\run.bat --help
  dist\main-standalone\run.bat
  eng.exe view build\result\result.engres

VS Code IDE preview:
  code --install-extension tools\englang-vscode-preview-$Version.vsix
  open a .eng file
  run "EngLang: Check Current File"

Generated artifacts are written under build\result in the current folder.
The curated user guide is docs\EngLang_User_Test_Guide.pdf. The language
grammar guide is docs\EngLang_Language_Grammar_Guide.pdf. Developer markdown
docs are kept in the source repository and are not bundled into this package.
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
    $PublicVersion = Get-PublicVersion
    $ZipPath = Join-Path $RepoRoot "dist\englang-preview-v$PublicVersion-windows-x64.zip"
    $KoreanWord = -join @([char]0xD55C, [char]0xAE00)
    $SmokeRoot = Join-Path $RepoRoot "dist\portable smoke $KoreanWord"
    Remove-Item -LiteralPath $SmokeRoot -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $SmokeRoot | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $SmokeRoot -Force
    $Eng = Join-Path $SmokeRoot "eng.exe"
    $Lsp = Join-Path $SmokeRoot "eng-lsp.exe"

    Push-Location $SmokeRoot
    try {
        Invoke-Native $Eng "doctor"
        Invoke-Native (Join-Path $SmokeRoot "eng-ide.exe") "--smoke"
        Invoke-Native $Lsp "--smoke"
        Invoke-Native $Eng "run" "examples\official\01_csv_plot\main.eng" "--save-artifacts"
        Invoke-Native $Eng "view" "build\result\result.engres"
        Invoke-Native $Eng "run" "examples\official\02_simple_system\main.eng" "--save-artifacts"
        if (-not (Test-Path (Join-Path $SmokeRoot "build\result\report_spec.json"))) {
            throw "portable smoke did not create build\result\report_spec.json"
        }
        Invoke-Native $Eng "run" "examples\official\03_integrated_hvac\main.eng" "--save-artifacts"
        $IntegratedResult = Get-Content -LiteralPath (Join-Path $SmokeRoot "build\result\result.engres") -Raw
        $IntegratedPlotSpec = Get-Content -LiteralPath (Join-Path $SmokeRoot "build\result\plots\plot_spec.json") -Raw
        if (-not $IntegratedResult.Contains('"policy_results"') -or -not $IntegratedResult.Contains('"solver_result"') -or -not $IntegratedPlotSpec.Contains("Integrated HVAC coil heat rate")) {
            throw "portable smoke integrated HVAC example did not produce expected policy, solver, and plot artifacts"
        }
        Invoke-Native $Eng "build" "examples\official\01_csv_plot\main.eng" "--standalone" "--profile" "repro"
        $StandaloneRunner = Join-Path $SmokeRoot "dist\main-standalone\run.bat"
        if (-not (Test-Path $StandaloneRunner)) {
            throw "portable smoke did not create dist\main-standalone\run.bat"
        }
        Invoke-Native $StandaloneRunner "--help"
        Invoke-Native $StandaloneRunner
        if (-not (Test-Path (Join-Path $SmokeRoot "dist\main-standalone\build\result\plots\plot_spec.json"))) {
            throw "standalone packaged runner did not create PlotSpec artifacts"
        }
        $StandaloneDir = Join-Path $SmokeRoot "dist\main-standalone"
        Copy-Item -LiteralPath (Join-Path $StandaloneDir "source\data\sensor.csv") -Destination (Join-Path $StandaloneDir "source\data\sensor_override.csv") -Force
        Invoke-Native $StandaloneRunner "--input" "data/sensor_override.csv"
        $StandaloneResult = Read-ArtifactJson (Join-Path $StandaloneDir "build\result\result.engres")
        $StandaloneInputArg = @($StandaloneResult.arg_values | Where-Object { $_.name -eq "input" })[0]
        if ($StandaloneInputArg.value -ne "data/sensor_override.csv" -or $StandaloneInputArg.source -ne "cli") {
            throw "standalone packaged runner did not record non-default Args override"
        }
        $Version = Get-WorkspaceVersion
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\extension.js"))) {
            throw "portable package did not include VS Code extension source"
        }
        if (-not (Test-Path $Lsp)) {
            throw "portable package did not include eng-lsp.exe"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\englang-vscode-preview-$Version.vsix"))) {
            throw "portable package did not include VS Code VSIX"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "docs\EngLang_User_Test_Guide.pdf"))) {
            throw "portable package did not include user guide PDF"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "docs\EngLang_Language_Grammar_Guide.pdf"))) {
            throw "portable package did not include language grammar guide PDF"
        }
        $BundledMarkdownDocs = @(Get-ChildItem -LiteralPath (Join-Path $SmokeRoot "docs") -Recurse -Filter "*.md" -ErrorAction SilentlyContinue)
        if ($BundledMarkdownDocs.Count -gt 0) {
            throw "portable package docs folder should contain curated release docs, not developer markdown files"
        }
    } finally {
        Pop-Location
    }

    Write-Host "Portable package smoke passed at $SmokeRoot"
}

function Invoke-ReleaseCheck {
    Invoke-Ci
    Invoke-DocsCheck
    Invoke-IdeCheck
    Invoke-ArtifactsCheck
    Invoke-PackageSmoke
    $Version = Get-WorkspaceVersion
    $PublicVersion = Get-PublicVersion
    $ZipPath = Join-Path $RepoRoot "dist\englang-preview-v$PublicVersion-windows-x64.zip"
    $ChecksumPath = "$ZipPath.sha256"
    if (-not (Test-Path $ZipPath)) {
        throw "release check did not create $ZipPath"
    }
    if (-not (Test-Path $ChecksumPath)) {
        throw "release check did not create $ChecksumPath"
    }
    $GuidePath = Join-Path $RepoRoot "dist\englang-user-test-guide-v$PublicVersion.pdf"
    if (-not (Test-Path $GuidePath)) {
        throw "release check did not create $GuidePath"
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

version = $PublicVersion
workspace_package_version = $Version
commit = $GitCommit
zip = $(Split-Path -Leaf $ZipPath)
user_guide = $(Split-Path -Leaf $GuidePath)
sha256 = $ActualHash

verified:
  dev.bat ci
  dev.bat jit-check
  dev.bat docs-check
  dev.bat ide-check
  dev.bat artifacts-check
  dev.bat package-smoke
  standalone packaged runner
  eng-ide.exe smoke
  eng-lsp.exe smoke
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

function Invoke-CleanGenerated {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $RepoRoot "build")
    $DistRoot = Join-Path $RepoRoot "dist"
    if (Test-Path -LiteralPath $DistRoot -PathType Container) {
        Get-ChildItem -LiteralPath $DistRoot -Force | Where-Object { $_.Name -ne "dev-current" } | ForEach-Object {
            Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Show-Help {
    Write-Host @"
EngLang development wrapper

Usage:
  .\dev.bat setup          Install pinned local Rust and portable Python/oodocs in .dev, then build
  .\dev.bat doctor         Run eng doctor through the local toolchain
  .\dev.bat build          Build the Rust workspace
  .\dev.bat test           Run Rust tests and EngLang example smoke tests
  .\dev.bat fmt            Format Rust code
  .\dev.bat clippy         Run clippy with warnings denied
  .\dev.bat ci             Run fmt, tests, clippy, and preview example
  .\dev.bat docs-check     Check supported documentation Eng snippets
  .\dev.bat grammar-docs   Generate the oodocs language grammar PDF
  .\dev.bat ide-check      Validate the VS Code extension preview
  .\dev.bat lsp-check      Validate eng-lsp.exe stdio, smoke, and snapshot output
  .\dev.bat jit-check      Validate runtime optimization track kernel planning and bench output
  .\dev.bat ide            Run the native EngLang tester IDE
  .\dev.bat dev-current    Build latest release test IDE into dist\dev-current
  .\dev.bat artifacts-check Validate artifact schemas and golden baselines
  .\dev.bat run-example    Run examples\official\01_csv_plot\main.eng
  .\dev.bat package        Build release, assemble dist\englang-preview, zip it, and write SHA256
  .\dev.bat package-smoke  Extract the portable zip under a Korean/space path and smoke it
  .\dev.bat release-check  Run full local release gate and verify checksum
  .\dev.bat clean-generated Remove build and transient dist outputs, preserving dist\dev-current
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
    "grammar-docs" { Invoke-GrammarDocs }
    "ide-check" { Invoke-IdeCheck }
    "lsp-check" { Invoke-LspCheck }
    "jit-check" { Invoke-JitCheck }
    "ide" { Invoke-Ide }
    "dev-current" { Invoke-DevCurrent }
    "artifacts-check" { Invoke-ArtifactsCheck }
    "run-example" { Invoke-RunExample }
    "package" { Invoke-Package }
    "package-smoke" { Invoke-PackageSmoke }
    "release-check" { Invoke-ReleaseCheck }
    "clean-generated" { Invoke-CleanGenerated }
    "clean" { Invoke-Clean }
    default { Show-Help }
}
