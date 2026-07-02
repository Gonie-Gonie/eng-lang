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

function Test-IsPreviewVersion {
    param([string] $Version = (Get-WorkspaceVersion))
    return $Version.EndsWith("-preview")
}

function Get-PackageRootName {
    if (Test-IsPreviewVersion) {
        return "englang-preview"
    }
    return "englang"
}

function Get-ZipFileName {
    $PublicVersion = Get-PublicVersion
    if (Test-IsPreviewVersion) {
        return "englang-preview-v$PublicVersion-windows-x64.zip"
    }
    return "englang-v$PublicVersion-windows-x64.zip"
}

function Get-UserGuideFileName {
    $PublicVersion = Get-PublicVersion
    if (Test-IsPreviewVersion) {
        return "englang-user-test-guide-v$PublicVersion.pdf"
    }
    return "englang-user-guide-v$PublicVersion.pdf"
}

function Get-PackageUserGuideFileName {
    if (Test-IsPreviewVersion) {
        return "EngLang_User_Test_Guide.pdf"
    }
    return "EngLang_User_Guide.pdf"
}

function Get-VsixFileName {
    $Version = Get-WorkspaceVersion
    if (Test-IsPreviewVersion -Version $Version) {
        return "englang-vscode-preview-$Version.vsix"
    }
    return "englang-vscode-$Version.vsix"
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
    Write-Host "Tauri IDE uses static HTML/CSS/JS assets; Node/npm is not required."
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

function Invoke-WorkflowsTest {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    $CliRunSource = Get-Content -LiteralPath (Join-Path $RepoRoot "crates\eng_cli\src\main.rs") -Raw
    $CliRunDocs = Get-Content -LiteralPath (Join-Path $RepoRoot "docs\reference\cli\run.md") -Raw
    foreach ($RequiredCliRunLabel in @(
        "result data",
        "review data",
        "static run graph",
        "run graph",
        "reproducibility lock",
        "run log",
        "external process results",
        "cache records",
        "test results",
        "report data",
        "plot svg",
        "plot data",
        "plot output list",
        "output list",
        "report html"
    )) {
        if (-not $CliRunSource.Contains($RequiredCliRunLabel) -or -not $CliRunDocs.Contains($RequiredCliRunLabel)) {
            throw "CLI run output and docs must expose user-facing artifact label '$RequiredCliRunLabel'"
        }
    }
    foreach ($ForbiddenCliRunLabel in @(
        "staticplan:",
        "runplan:",
        "runlock:",
        "runlog:",
        "reportspec:",
        "plotspec:",
        "plotmanifest:",
        "process:  ",
        "cache:    ",
        "tests:    ",
        "outputs:  "
    )) {
        if ($CliRunSource.Contains($ForbiddenCliRunLabel) -or $CliRunDocs.Contains($ForbiddenCliRunLabel)) {
            throw "CLI run output should use user-facing artifact labels instead of '$ForbiddenCliRunLabel'"
        }
    }
    Invoke-Native $cargo "test" "-p" "eng_compiler" "workflow_modules"
    Invoke-Native $cargo "test" "-p" "eng_runtime" "workflow_modules"
    $WorkflowRoot = Join-Path $RepoRoot "examples\workflows"
    $WorkflowSourcePaths = @(Get-ChildItem -LiteralPath $WorkflowRoot -Directory | ForEach-Object {
        Join-Path $_.FullName "main.eng"
    } | Where-Object {
        Test-Path -LiteralPath $_ -PathType Leaf
    } | Sort-Object)
    if ($WorkflowSourcePaths.Count -lt 3) {
        throw "Native workflow smoke expected at least workflow 01/02/03 main.eng files"
    }
    $WorkflowPublicDocPaths = @(Get-ChildItem -LiteralPath $WorkflowRoot -Recurse -File -Include "*.md", "*.txt" | Sort-Object FullName)
    foreach ($WorkflowPublicDocPath in $WorkflowPublicDocPaths) {
        $WorkflowPublicDoc = Get-Content -LiteralPath $WorkflowPublicDocPath.FullName -Raw
        foreach ($ForbiddenWorkflowDocWording in @(
            "files produced by an external process"
        )) {
            if ($WorkflowPublicDoc.Contains($ForbiddenWorkflowDocWording)) {
                throw "Native workflow public docs must not describe native artifacts as external-process output: $($WorkflowPublicDocPath.FullName)"
            }
        }
    }
    foreach ($WorkflowSourcePath in $WorkflowSourcePaths) {
        $Workflow = $WorkflowSourcePath.Substring($RepoRoot.Length).TrimStart('\')
        $WorkflowSource = Get-Content -LiteralPath $WorkflowSourcePath -Raw
        if ($WorkflowSource -match "(?im)\brun\s+command\b") {
            throw "Native workflow source must not use run command: $Workflow"
        }
        foreach ($PythonMarker in @(
            "\bpython(?:\d+(?:\.\d+)*)?(?:\.exe)?\b",
            "\.py\b",
            "\bsubprocess\b",
            "\bpandas\b",
            "\bnumpy\b",
            "\bmatplotlib\b",
            "\bjupyter\b",
            "\bnotebook\b"
        )) {
            if ($WorkflowSource -match "(?i)$PythonMarker") {
                throw "Native workflow source must not contain Python/notebook marker $PythonMarker`: $Workflow"
            }
        }
        if ($WorkflowSource -match "(?i)\bselect_first_row\s*\(") {
            throw "Native workflow source must use filter + require_one instead of legacy select_first_row: $Workflow"
        }
        if ($Workflow -like "*01_weather_api_to_standard_file*") {
            $WorkflowPublicTextPaths = @(
                $WorkflowSourcePath,
                (Join-Path $RepoRoot "examples\workflows\01_weather_api_to_standard_file\README.md"),
                (Join-Path $RepoRoot "examples\workflows\01_weather_api_to_standard_file\expected\review_summary.md"),
                (Join-Path $RepoRoot "examples\workflows\README.md")
            )
            foreach ($WorkflowPublicTextPath in $WorkflowPublicTextPaths) {
                $WorkflowPublicText = Get-Content -LiteralPath $WorkflowPublicTextPath -Raw
                foreach ($ForbiddenWorkflowWording in @(
                    "api_fixture",
                    "Weather fixture",
                    "fixture fetched",
                    "network/cache fixture",
                    "HTTP fixture",
                    "schema StationMap with two fixture rows"
                )) {
                    if ($WorkflowPublicText.Contains($ForbiddenWorkflowWording)) {
                        throw "Workflow 01 public wording should describe pinned offline responses instead of '$ForbiddenWorkflowWording': $WorkflowPublicTextPath"
                    }
                }
            }
        }
        Invoke-Native $cargo "run" "-p" "eng_cli" "--" "run" $Workflow "--save-artifacts"
        $ProcessResultsPath = Join-Path $RepoRoot "build\result\process_results.json"
        if (-not (Test-Path -LiteralPath $ProcessResultsPath)) {
            throw "Native workflow smoke must write process_results.json: $Workflow"
        }
        $ProcessResults = Get-Content -LiteralPath $ProcessResultsPath -Raw | ConvertFrom-Json
        $ProcessCount = 0
        if ($null -ne $ProcessResults.process_count) {
            $ProcessCount = [int]$ProcessResults.process_count
        }
        $ProcessListCount = 0
        if ($null -ne $ProcessResults.processes) {
            $ProcessListCount = @($ProcessResults.processes).Count
        }
        if ($ProcessCount -ne 0 -or $ProcessListCount -ne 0) {
            throw "Native workflow smoke must not execute external processes: $Workflow"
        }
    }
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

function Test-MarkdownLinks {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Files
    )

    $failures = New-Object System.Collections.Generic.List[string]
    foreach ($file in $Files) {
        $text = Get-Content -LiteralPath $file -Raw -Encoding UTF8
        $matches = [regex]::Matches($text, '\[[^\]]+\]\(([^)#]+)(?:#[^)]+)?\)')
        foreach ($match in $matches) {
            $target = $match.Groups[1].Value.Trim()
            if ($target -eq "" -or $target -match '^(https?:|mailto:|#)') {
                continue
            }
            if ($target.StartsWith("<") -and $target.EndsWith(">")) {
                $target = $target.Substring(1, $target.Length - 2)
            }
            $target = [System.Uri]::UnescapeDataString($target)
            $candidate = Join-Path (Split-Path -Parent $file) $target
            if (-not (Test-Path -LiteralPath $candidate)) {
                $relative = $file.Substring($RepoRoot.Length).TrimStart('\')
                $failures.Add("$relative links to missing target $target") | Out-Null
            }
        }
    }

    if ($failures.Count -gt 0) {
        $failures | Sort-Object | ForEach-Object { Write-Host $_ }
        throw "Docs link check failed with $($failures.Count) missing target(s)."
    }

    Write-Host "Docs link check passed. Checked $($Files.Count) Markdown file(s)."
}

function Test-ModuleRegistryDocs {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RegistryPath,

        [Parameter(Mandatory = $true)]
        [string] $ReadmePath,

        [Parameter(Mandatory = $true)]
        [string] $WorkflowDocsPath
    )

    if (-not (Test-Path -LiteralPath $RegistryPath -PathType Leaf)) {
        throw "missing module registry at $RegistryPath"
    }
    if (-not (Test-Path -LiteralPath $ReadmePath -PathType Leaf)) {
        throw "missing stdlib README at $ReadmePath"
    }
    if (-not (Test-Path -LiteralPath $WorkflowDocsPath -PathType Leaf)) {
        throw "missing workflow module docs at $WorkflowDocsPath"
    }

    $RegistryText = Get-Content -LiteralPath $RegistryPath -Raw -Encoding UTF8
    $ReadmeText = Get-Content -LiteralPath $ReadmePath -Raw -Encoding UTF8
    $WorkflowDocsText = Get-Content -LiteralPath $WorkflowDocsPath -Raw -Encoding UTF8
    $RegistryModules = [System.Collections.Generic.HashSet[string]]::new()
    $RegistryEntries = New-Object System.Collections.Generic.List[object]
    $CurrentEntry = $null
    foreach ($match in [regex]::Matches($RegistryText, '(?m)^\[module\."([^"]+)"\]')) {
        [void]$RegistryModules.Add($match.Groups[1].Value)
    }
    foreach ($rawLine in ($RegistryText -split "`r?`n")) {
        $line = $rawLine.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) {
            continue
        }
        if ($line -match '^\[module\."([^"]+)"\]') {
            if ($null -ne $CurrentEntry) {
                $RegistryEntries.Add($CurrentEntry) | Out-Null
            }
            $CurrentEntry = [ordered]@{ name = $Matches[1] }
            continue
        }
        if ($null -eq $CurrentEntry) {
            continue
        }
        if ($line -match '^(status|backing|purpose)\s*=\s*"([^"]*)"') {
            $CurrentEntry[$Matches[1]] = $Matches[2]
            continue
        }
        if ($line -match '^(artifacts|diagnostics|examples|tests)\s*=\s*\[(.*)\]') {
            $items = New-Object System.Collections.Generic.List[string]
            foreach ($itemMatch in [regex]::Matches($Matches[2], '"([^"]*)"')) {
                $items.Add($itemMatch.Groups[1].Value) | Out-Null
            }
            $CurrentEntry[$Matches[1]] = $items.ToArray()
        }
    }
    if ($null -ne $CurrentEntry) {
        $RegistryEntries.Add($CurrentEntry) | Out-Null
    }
    if ($RegistryModules.Count -eq 0) {
        throw "module registry has no [module.`"eng.name`"] entries"
    }
    foreach ($entry in $RegistryEntries) {
        foreach ($field in @("status", "backing", "purpose", "artifacts", "diagnostics", "examples", "tests")) {
            if (-not $entry.Contains($field)) {
                throw "module registry entry $($entry.name) is missing $field"
            }
        }
    }

    $ReadmeModules = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($match in [regex]::Matches($ReadmeText, '`(eng\.[a-z0-9_]+)`')) {
        [void]$ReadmeModules.Add($match.Groups[1].Value)
    }

    $MissingFromReadme = $RegistryModules | Where-Object { -not $ReadmeModules.Contains($_) } | Sort-Object
    $MissingFromRegistry = $ReadmeModules | Where-Object { -not $RegistryModules.Contains($_) } | Sort-Object
    if ($MissingFromReadme.Count -gt 0 -or $MissingFromRegistry.Count -gt 0) {
        if ($MissingFromReadme.Count -gt 0) {
            Write-Host "Registry modules missing from stdlib README:"
            $MissingFromReadme | ForEach-Object { Write-Host "  $_" }
        }
        if ($MissingFromRegistry.Count -gt 0) {
            Write-Host "Stdlib README modules missing from registry:"
            $MissingFromRegistry | ForEach-Object { Write-Host "  $_" }
        }
        throw "Module registry docs check failed."
    }

    $generatedTable = New-Object System.Collections.Generic.List[string]
    $generatedTable.Add("| Module | Status | Backing | Artifacts | Diagnostics | Examples | Tests |") | Out-Null
    $generatedTable.Add("|---|---|---|---|---|---|---|") | Out-Null
    foreach ($entry in $RegistryEntries) {
        $status = Convert-ModuleRegistryStatusLabel $entry.status
        $backing = Convert-ModuleRegistryBackingLabel $entry.backing
        $artifacts = Convert-ModuleRegistryTableCell $entry.artifacts
        $diagnostics = Convert-ModuleRegistryTableCell $entry.diagnostics
        $examples = Convert-ModuleRegistryTableCell $entry.examples
        $tests = Convert-ModuleRegistryTableCell $entry.tests
        $generatedTable.Add("| ``$($entry.name)`` | $status | $backing | $artifacts | $diagnostics | $examples | $tests |") | Out-Null
    }
    $expectedBlock = ($generatedTable.ToArray() -join "`n").Trim()
    $startMarker = "<!-- module-registry-table:start -->"
    $endMarker = "<!-- module-registry-table:end -->"
    $startIndex = $WorkflowDocsText.IndexOf($startMarker)
    $endIndex = $WorkflowDocsText.IndexOf($endMarker)
    if ($startIndex -lt 0 -or $endIndex -lt 0 -or $endIndex -le $startIndex) {
        throw "workflow module docs must contain module-registry-table start/end markers"
    }
    $actualBlock = $WorkflowDocsText.Substring(
        $startIndex + $startMarker.Length,
        $endIndex - ($startIndex + $startMarker.Length)
    ).Trim() -replace "`r`n", "`n"
    if ($actualBlock -ne $expectedBlock) {
        Write-Host "docs/current/workflow_modules.md module table is out of sync with stdlib/eng/modules.toml."
        Write-Host "Expected generated table:"
        Write-Host $expectedBlock
        throw "Module registry docs check failed."
    }

    Write-Host "Module registry docs check passed. Checked $($RegistryModules.Count) module(s)."
}

function Convert-ModuleRegistryStatusLabel {
    param([string] $Status)
    switch ($Status) {
        "supported" { return "Supported" }
        "supported_narrow" { return "Supported narrow" }
        "native_preview" { return "Native workflow support" }
        "planned" { return "Planned" }
        "internal_planned" { return "Internal planned" }
        "internal" { return "Internal" }
        default { return $Status }
    }
}

function Convert-ModuleRegistryBackingLabel {
    param([string] $Backing)
    switch ($Backing) {
        "compiler_runtime_builtin" { return "Compiler/runtime" }
        "none" { return "No executable backing" }
        "internal" { return "Internal" }
        default { return $Backing }
    }
}

function Convert-ModuleRegistryTableCell {
    param([object[]] $Items)
    if ($null -eq $Items -or $Items.Count -eq 0) {
        return "-"
    }
    $formatted = foreach ($item in $Items) {
        "``$item``"
    }
    return ($formatted -join "<br>")
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
        "docs\reference",
        "docs\user",
        "docs\workflows",
        "docs\architecture",
        "docs\development",
        "docs\internal"
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

    $linkTargets = @("README.md", "LLM_CONTEXT.md", "docs")
    $linkFiles = New-Object System.Collections.Generic.List[string]
    foreach ($target in $linkTargets) {
        $path = Join-Path $RepoRoot $target
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $linkFiles.Add($path) | Out-Null
        } elseif (Test-Path -LiteralPath $path -PathType Container) {
            Get-ChildItem -LiteralPath $path -Recurse -Filter "*.md" | ForEach-Object {
                $linkFiles.Add($_.FullName) | Out-Null
            }
        }
    }
    Test-MarkdownLinks -Files $linkFiles.ToArray()
    Test-ModuleRegistryDocs `
        -RegistryPath (Join-Path $RepoRoot "stdlib\eng\modules.toml") `
        -ReadmePath (Join-Path $RepoRoot "stdlib\README.md") `
        -WorkflowDocsPath (Join-Path $RepoRoot "docs\current\workflow_modules.md")

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

function Assert-ArtifactNullableValue {
    param(
        $Actual,

        $Expected,

        [Parameter(Mandatory = $true)]
        [string] $Label
    )

    if ($null -eq $Expected) {
        Assert-Artifact ($null -eq $Actual) "$Label expected null but got $Actual"
    } else {
        Assert-ArtifactValue $Actual $Expected $Label
    }
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
        "docs\schemas\output_manifest.schema.json",
        "docs\schemas\static_run_plan.schema.json",
        "docs\schemas\run_plan.schema.json",
        "docs\schemas\run_lock.schema.json",
        "docs\schemas\run_log.schema.json",
        "docs\schemas\process_results.schema.json",
        "docs\schemas\test_results.schema.json",
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
    Assert-ArtifactValue $reviewPromotion.source_hash $Golden.review.csv_source_hash "review.csv_promotions[0].source_hash"
    Assert-ArtifactNumber @($review.axis_info).Count $Golden.review.axis_info_count "review.axis_info count"
    $reviewSeriesAxis = @($review.axis_info) | Where-Object { $_.binding -eq $Golden.review.timeseries_axis_binding } | Select-Object -First 1
    Assert-Artifact ($null -ne $reviewSeriesAxis) "review.axis_info missing $($Golden.review.timeseries_axis_binding)"
    Assert-ArtifactValue $reviewSeriesAxis.axis $Golden.review.timeseries_axis "review.axis_info TimeSeries axis"
    Assert-ArtifactValue $reviewSeriesAxis.role $Golden.review.timeseries_axis_role "review.axis_info TimeSeries role"
    $reviewIntegration = @($review.integrations)[0]
    Assert-ArtifactValue $reviewIntegration.binding $Golden.review.integration_binding "review.integrations[0].binding"
    Assert-ArtifactValue $reviewIntegration.over_axis $Golden.review.integration_over_axis "review.integrations[0].over_axis"
    Assert-ArtifactValue $reviewIntegration.result_quantity $Golden.review.integration_result_quantity "review.integrations[0].result_quantity"
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
    Assert-ArtifactNumber @($reportSpec.time_axes).Count $Golden.report_spec.time_axis_count "report_spec.time_axes count"
    $reportTimeAxis = @($reportSpec.time_axes)[0]
    Assert-ArtifactValue $reportTimeAxis.name $Golden.report_spec.time_axis_name "report_spec.time_axes[0].name"
    Assert-ArtifactValue $reportTimeAxis.axis $Golden.report_spec.time_axis "report_spec.time_axes[0].axis"
    Assert-ArtifactValue $reportTimeAxis.unit $Golden.report_spec.time_axis_unit "report_spec.time_axes[0].unit"
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
    Assert-ArtifactValue $reportIntegration.over_axis $Golden.report_spec.integration_over_axis "report_spec.computed_integrations[0].over_axis"
    Assert-ArtifactValue $reportIntegration.result_quantity $Golden.report_spec.integration_result_quantity "report_spec.computed_integrations[0].result_quantity"
    Assert-ArtifactValue $reportIntegration.unit $Golden.report_spec.integration_unit "report_spec.computed_integrations[0].unit"
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
    Assert-ArtifactValue $resultDataHash.hash $Golden.result.csv_source_hash "result.provenance.data_hashes[0].hash"
    Assert-ArtifactNumber $result.object_store.scalar_count $Golden.result.scalar_count "result.object_store.scalar_count"
    Assert-ArtifactNumber $result.object_store.table_count $Golden.result.table_count "result.object_store.table_count"
    Assert-ArtifactNumber $result.object_store.timeseries_count $Golden.result.timeseries_count "result.object_store.timeseries_count"
    Assert-ArtifactNumber $result.provenance.schema_count $Golden.result.schema_count "result.provenance.schema_count"
    Assert-ArtifactNumber $result.provenance.csv_promotion_count $Golden.result.csv_promotion_count "result.provenance.csv_promotion_count"
    Assert-ArtifactNumber @($result.typed_payload.statistics).Count $Golden.result.statistics_count "result.typed_payload.statistics count"
    Assert-ArtifactNumber @($result.typed_payload.integrations).Count $Golden.result.integrations_count "result.typed_payload.integrations count"
    $tableObject = @($result.object_store.objects) | Where-Object { $_.name -eq "sensor" } | Select-Object -First 1
    Assert-Artifact ($null -ne $tableObject) "result.object_store.objects missing sensor table"
    Assert-ArtifactValue $tableObject.source_hash $Golden.result.csv_source_hash "result.sensor.source_hash"
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
    Assert-ArtifactValue $seriesObject.axis $Golden.result.timeseries_axis "result.Q_coil.axis"
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
    Assert-ArtifactValue $integrationPayload.over_axis $Golden.result.integration_over_axis "result.typed_payload.integrations[0].over_axis"
    Assert-ArtifactValue $integrationPayload.result_quantity $Golden.result.integration_result_quantity "result.typed_payload.integrations[0].result_quantity"
    Assert-ArtifactValue $integrationPayload.unit $Golden.result.integration_unit "result.typed_payload.integrations[0].unit"
    Assert-ArtifactFloat $integrationPayload.value $Golden.result.integration_value_j "result.typed_payload.integrations[0].value"
    Assert-ArtifactNumber @($result.typed_payload.time_axes).Count $Golden.result.time_axis_count "result.typed_payload.time_axes count"
    $resultTimeAxis = @($result.typed_payload.time_axes)[0]
    Assert-ArtifactValue $resultTimeAxis.name $Golden.result.time_axis_name "result.typed_payload.time_axes[0].name"
    Assert-ArtifactValue $resultTimeAxis.axis $Golden.result.time_axis "result.typed_payload.time_axes[0].axis"
    Assert-ArtifactValue $resultTimeAxis.unit $Golden.result.time_axis_unit "result.typed_payload.time_axes[0].unit"
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

    $runPlan = Read-ArtifactJson (Join-Path $RepoRoot "build\result\run_plan.json")
    Assert-ArtifactValue $runPlan.format "eng-run-plan-v1" "run_plan.format"
    Assert-ArtifactValue $runPlan.source_hash $review.source_hash "run_plan.source_hash"
    Assert-ArtifactValue $runPlan.rerun_status "executed" "run_plan.rerun_status"
    $runPlanNodeCount = @($runPlan.graph.nodes).Count
    $runPlanEdgeCount = @($runPlan.graph.edges).Count
    Assert-ArtifactNumber $runPlan.graph.node_count $runPlanNodeCount "run_plan.graph.node_count"
    Assert-ArtifactNumber $runPlan.graph.edge_count $runPlanEdgeCount "run_plan.graph.edge_count"
    $sourceNode = @($runPlan.graph.nodes) | Where-Object { $_.id -eq "source:program" } | Select-Object -First 1
    Assert-Artifact ($null -ne $sourceNode) "run_plan.graph.nodes missing source:program"
    Assert-ArtifactValue $sourceNode.status "loaded" "run_plan source node status"
    Assert-Artifact ([string]$runPlan.artifact_hashes.static_run_plan -ne "") "run_plan.artifact_hashes.static_run_plan is empty"
    Assert-ArtifactValue $review.workflow_graph.format "eng-workflow-graph-review-v1" "review.workflow_graph.format"
    Assert-ArtifactValue $review.workflow_graph.source "run_plan" "review.workflow_graph.source"
    Assert-ArtifactNumber $review.workflow_graph.node_count $runPlanNodeCount "review.workflow_graph.node_count"
    Assert-ArtifactNumber $review.workflow_graph.edge_count $runPlanEdgeCount "review.workflow_graph.edge_count"
    Assert-ArtifactNumber @($review.workflow_graph.risk_by_node).Count $runPlanNodeCount "review.workflow_graph.risk_by_node count"

    $staticRunPlan = Read-ArtifactJson (Join-Path $RepoRoot "build\result\static_run_plan.json")
    Assert-ArtifactValue $staticRunPlan.format "eng-static-run-plan-v1" "static_run_plan.format"
    Assert-ArtifactValue $staticRunPlan.execution_stage "pre_execution" "static_run_plan.execution_stage"
    Assert-ArtifactValue $staticRunPlan.status "planned" "static_run_plan.status"
    Assert-ArtifactValue $staticRunPlan.source_hash $review.source_hash "static_run_plan.source_hash"
    Assert-ArtifactNumber $staticRunPlan.graph.node_count @($staticRunPlan.graph.nodes).Count "static_run_plan.graph.node_count"
    Assert-ArtifactNumber $staticRunPlan.graph.edge_count @($staticRunPlan.graph.edges).Count "static_run_plan.graph.edge_count"
    $staticSourceNode = @($staticRunPlan.graph.nodes) | Where-Object { $_.id -eq "source:program" } | Select-Object -First 1
    Assert-Artifact ($null -ne $staticSourceNode) "static_run_plan.graph.nodes missing source:program"

    $runLock = Read-ArtifactJson (Join-Path $RepoRoot "build\result\run_lock.json")
    Assert-ArtifactValue $runLock.format "eng-run-lock-v1" "run_lock.format"
    Assert-ArtifactValue $runLock.source_hash $review.source_hash "run_lock.source_hash"
    Assert-ArtifactValue $runLock.execution_profile "normal" "run_lock.execution_profile"
    Assert-ArtifactValue $runLock.rerun_decision.decision "run" "run_lock.rerun_decision.decision"
    Assert-Artifact ([string]$runLock.input_hash -ne "") "run_lock.input_hash is empty"
    Assert-Artifact ([string]$runLock.artifact_hashes.static_run_plan -ne "") "run_lock.artifact_hashes.static_run_plan is empty"
    Assert-Artifact ([string]$runLock.artifact_hashes.run_plan -ne "") "run_lock.artifact_hashes.run_plan is empty"

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
    Assert-ArtifactNumber @($review.simulation_results).Count $Golden.review.simulation_result_count "system review.simulation_results count"
    $reviewSimulation = @($review.simulation_results)[0]
    Assert-ArtifactValue $reviewSimulation.system $Golden.review.simulation_system "system review.simulation_results[0].system"
    Assert-ArtifactValue $reviewSimulation.status $Golden.review.simulation_status "system review.simulation_results[0].status"
    Assert-ArtifactValue $reviewSimulation.method $Golden.review.simulation_method "system review.simulation_results[0].method"
    Assert-ArtifactNumber @($reviewSimulation.variables.states).Count $Golden.review.simulation_state_count "system review simulation state count"
    Assert-ArtifactValue @($reviewSimulation.variables.states)[0] $Golden.review.simulation_state_name "system review simulation state name"
    Assert-ArtifactNumber @($reviewSimulation.variables.algebraic_variables).Count $Golden.review.simulation_algebraic_variable_count "system review simulation algebraic variable count"
    Assert-ArtifactNumber @($reviewSimulation.variables.inputs).Count $Golden.review.simulation_input_count "system review simulation input count"
    Assert-ArtifactValue @($reviewSimulation.variables.inputs)[0] $Golden.review.simulation_first_input "system review simulation first input"
    Assert-ArtifactValue @($reviewSimulation.variables.inputs)[1] $Golden.review.simulation_second_input "system review simulation second input"
    Assert-ArtifactNumber @($reviewSimulation.variables.parameters).Count $Golden.review.simulation_parameter_count "system review simulation parameter count"
    Assert-ArtifactValue @($reviewSimulation.variables.parameters)[0] $Golden.review.simulation_first_parameter "system review simulation first parameter"
    Assert-ArtifactValue @($reviewSimulation.variables.parameters)[1] $Golden.review.simulation_second_parameter "system review simulation second parameter"
    Assert-ArtifactNumber @($reviewSimulation.variables.outputs).Count $Golden.review.simulation_output_count "system review simulation output count"
    Assert-ArtifactValue @($reviewSimulation.variables.outputs)[0] $Golden.review.simulation_output_name "system review simulation output name"
    Assert-ArtifactFloat $reviewSimulation.diagnostics.tolerance $Golden.review.simulation_tolerance "system review simulation tolerance"
    Assert-ArtifactNumber $reviewSimulation.diagnostics.max_iterations $Golden.review.simulation_max_iterations "system review simulation max_iterations"
    Assert-ArtifactNumber $reviewSimulation.diagnostics.iteration_count $Golden.review.simulation_iteration_count "system review simulation iteration_count"
    Assert-ArtifactValue $reviewSimulation.diagnostics.convergence_status $Golden.review.simulation_convergence_status "system review simulation convergence_status"
    Assert-Artifact ($null -eq $reviewSimulation.diagnostics.failure_reason) "system review simulation failure_reason should be null"
    Assert-ArtifactFloat $reviewSimulation.time_grid.duration $Golden.review.simulation_duration_s "system review simulation duration"
    Assert-ArtifactFloat $reviewSimulation.time_grid.timestep $Golden.review.simulation_time_step_s "system review simulation timestep"
    Assert-ArtifactNumber $reviewSimulation.time_grid.step_count $Golden.review.simulation_step_count "system review simulation step_count"
    $reviewSimulationState = @($reviewSimulation.states)[0]
    Assert-ArtifactValue $reviewSimulationState.name $Golden.review.simulation_state_name "system review simulation states[0].name"
    Assert-ArtifactNumber $reviewSimulationState.point_count $Golden.review.simulation_point_count "system review simulation state point_count"
    Assert-ArtifactFloat $reviewSimulationState.final_value $Golden.review.simulation_final_temp_deg_c "system review simulation final_value"

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
    Assert-ArtifactNumber @($reportSystemIr.solver_results).Count $Golden.report_spec.solver_result_count "system report_spec solver_results count"
    $reportSolverResult = @($reportSystemIr.solver_results)[0]
    Assert-ArtifactValue $reportSolverResult.status $Golden.report_spec.solver_result_status "system report_spec solver_results[0].status"
    Assert-ArtifactNumber @($reportSolverResult.states).Count $Golden.report_spec.solver_state_count "system report_spec solver_results[0] state count"
    Assert-ArtifactValue @($reportSolverResult.states)[0] $Golden.report_spec.solver_state_name "system report_spec solver_results[0] state name"
    Assert-ArtifactNumber @($reportSolverResult.algebraic_variables).Count $Golden.report_spec.solver_algebraic_variable_count "system report_spec solver_results[0] algebraic variable count"
    Assert-ArtifactNumber @($reportSolverResult.inputs).Count $Golden.report_spec.solver_input_count "system report_spec solver_results[0] input count"
    Assert-ArtifactValue @($reportSolverResult.inputs)[0] $Golden.report_spec.solver_first_input "system report_spec solver_results[0] first input"
    Assert-ArtifactValue @($reportSolverResult.inputs)[1] $Golden.report_spec.solver_second_input "system report_spec solver_results[0] second input"
    Assert-ArtifactNumber @($reportSolverResult.parameters).Count $Golden.report_spec.solver_parameter_count "system report_spec solver_results[0] parameter count"
    Assert-ArtifactValue @($reportSolverResult.parameters)[0] $Golden.report_spec.solver_first_parameter "system report_spec solver_results[0] first parameter"
    Assert-ArtifactValue @($reportSolverResult.parameters)[1] $Golden.report_spec.solver_second_parameter "system report_spec solver_results[0] second parameter"
    Assert-ArtifactNumber @($reportSolverResult.outputs).Count $Golden.report_spec.solver_output_count "system report_spec solver_results[0] output count"
    Assert-ArtifactValue @($reportSolverResult.outputs)[0] $Golden.report_spec.solver_output_name "system report_spec solver_results[0] output name"
    Assert-ArtifactFloat $reportSolverResult.tolerance $Golden.report_spec.solver_tolerance "system report_spec solver_results[0].tolerance"
    Assert-ArtifactNumber $reportSolverResult.max_iterations $Golden.report_spec.solver_max_iterations "system report_spec solver_results[0].max_iterations"
    Assert-ArtifactNumber $reportSolverResult.iteration_count $Golden.report_spec.solver_iteration_count "system report_spec solver_results[0].iteration_count"
    Assert-ArtifactValue $reportSolverResult.convergence_status $Golden.report_spec.solver_convergence_status "system report_spec solver_results[0].convergence_status"
    Assert-Artifact ($null -eq $reportSolverResult.failure_reason) "system report_spec solver_results[0].failure_reason should be null"
    $reportSolverPoints = @($reportSolverResult.points)
    Assert-ArtifactNumber $reportSolverPoints.Count $Golden.report_spec.solver_point_count "system report_spec solver_results[0].points count"
    Assert-ArtifactFloat $reportSolverResult.initial_value $Golden.report_spec.solver_initial_temp_deg_c "system report_spec solver_results[0].initial_value"
    Assert-ArtifactFloat $reportSolverResult.final_value $Golden.report_spec.solver_final_temp_deg_c "system report_spec solver_results[0].final_value"

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
    Assert-ArtifactNumber @($resultSystemPayload.solver_result.states).Count $Golden.result.solver_state_count "system result.solver_result state count"
    Assert-ArtifactValue @($resultSystemPayload.solver_result.states)[0] $Golden.result.solver_state_name "system result.solver_result state name"
    Assert-ArtifactNumber @($resultSystemPayload.solver_result.algebraic_variables).Count $Golden.result.solver_algebraic_variable_count "system result.solver_result algebraic variable count"
    Assert-ArtifactNumber @($resultSystemPayload.solver_result.inputs).Count $Golden.result.solver_input_count "system result.solver_result input count"
    Assert-ArtifactValue @($resultSystemPayload.solver_result.inputs)[0] $Golden.result.solver_first_input "system result.solver_result first input"
    Assert-ArtifactValue @($resultSystemPayload.solver_result.inputs)[1] $Golden.result.solver_second_input "system result.solver_result second input"
    Assert-ArtifactNumber @($resultSystemPayload.solver_result.parameters).Count $Golden.result.solver_parameter_count "system result.solver_result parameter count"
    Assert-ArtifactValue @($resultSystemPayload.solver_result.parameters)[0] $Golden.result.solver_first_parameter "system result.solver_result first parameter"
    Assert-ArtifactValue @($resultSystemPayload.solver_result.parameters)[1] $Golden.result.solver_second_parameter "system result.solver_result second parameter"
    Assert-ArtifactNumber @($resultSystemPayload.solver_result.outputs).Count $Golden.result.solver_output_count "system result.solver_result output count"
    Assert-ArtifactValue @($resultSystemPayload.solver_result.outputs)[0] $Golden.result.solver_output_name "system result.solver_result output name"
    Assert-ArtifactFloat $resultSystemPayload.solver_result.tolerance $Golden.result.solver_tolerance "system result.solver_result.tolerance"
    Assert-ArtifactNumber $resultSystemPayload.solver_result.max_iterations $Golden.result.solver_max_iterations "system result.solver_result.max_iterations"
    Assert-ArtifactNumber $resultSystemPayload.solver_result.iteration_count $Golden.result.solver_iteration_count "system result.solver_result.iteration_count"
    Assert-ArtifactValue $resultSystemPayload.solver_result.convergence_status $Golden.result.solver_convergence_status "system result.solver_result.convergence_status"
    Assert-Artifact ($null -eq $resultSystemPayload.solver_result.failure_reason) "system result.solver_result.failure_reason should be null"
    $resultSolverPoints = @($resultSystemPayload.solver_result.points)
    Assert-ArtifactNumber $resultSolverPoints.Count $Golden.result.solver_point_count "system result.solver_result.points count"
    Assert-ArtifactFloat $resultSystemPayload.solver_result.initial_value $Golden.result.solver_initial_temp_deg_c "system result.solver_result.initial_value"
    Assert-ArtifactFloat @($resultSolverPoints[0])[0] $Golden.result.solver_first_point_time_s "system result.solver_result.points[0].time"
    Assert-ArtifactFloat @($resultSolverPoints[0])[1] $Golden.result.solver_first_point_temp_deg_c "system result.solver_result.points[0].value"
    Assert-ArtifactFloat @($resultSolverPoints[$resultSolverPoints.Count - 1])[0] $Golden.result.solver_last_point_time_s "system result.solver_result last point time"
}

function Assert-MeasuredVsSimulatedGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--profile" $Golden.profile "--save-artifacts"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "measured review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "measured review.review_schema_version"
    Assert-ArtifactValue (Get-NormalizedArtifactPath $review.source_path) (Get-NormalizedArtifactPath $Golden.source) "measured review.source_path"
    Assert-ArtifactNumber @($review.csv_promotions).Count $Golden.review.csv_promotion_count "measured review.csv_promotions count"
    $reviewWeather = @($review.csv_promotions) | Where-Object { $_.binding -eq $Golden.review.weather_binding } | Select-Object -First 1
    $reviewMeasured = @($review.csv_promotions) | Where-Object { $_.binding -eq $Golden.review.measured_binding } | Select-Object -First 1
    Assert-Artifact ($null -ne $reviewWeather) "measured review missing weather CSV promotion"
    Assert-Artifact ($null -ne $reviewMeasured) "measured review missing measured CSV promotion"
    Assert-ArtifactValue $reviewWeather.source_hash $Golden.review.weather_source_hash "measured review weather source_hash"
    Assert-ArtifactValue $reviewMeasured.source_hash $Golden.review.measured_source_hash "measured review measured source_hash"
    $reviewMetric = @($review.variable_table) | Where-Object { $_.name -eq $Golden.review.metric_binding } | Select-Object -First 1
    Assert-Artifact ($null -ne $reviewMetric) "measured review missing RMSE variable"
    Assert-ArtifactValue $reviewMetric.quantity_kind $Golden.review.metric_quantity "measured review RMSE quantity"
    Assert-ArtifactValue $reviewMetric.display_unit $Golden.review.metric_unit "measured review RMSE unit"
    $reviewValidation = @($review.command_styles) | Where-Object { $_.canonical -eq $Golden.review.validation_canonical } | Select-Object -First 1
    Assert-Artifact ($null -ne $reviewValidation) "measured review missing validation command"
    Assert-ArtifactNumber @($review.simulation_results).Count $Golden.review.simulation_result_count "measured review.simulation_results count"
    $reviewSimulation = @($review.simulation_results)[0]
    Assert-ArtifactValue $reviewSimulation.system $Golden.review.simulation_system "measured review simulation system"
    Assert-ArtifactValue $reviewSimulation.binding $Golden.review.simulation_binding "measured review simulation binding"
    Assert-ArtifactValue $reviewSimulation.status $Golden.review.simulation_status "measured review simulation status"
    Assert-ArtifactValue $reviewSimulation.method $Golden.review.simulation_method "measured review simulation method"
    Assert-ArtifactNumber $reviewSimulation.time_grid.step_count $Golden.review.simulation_step_count "measured review simulation step_count"
    Assert-ArtifactValue $reviewSimulation.diagnostics.convergence_status $Golden.review.simulation_convergence_status "measured review simulation convergence_status"
    Assert-ArtifactNumber @($reviewSimulation.solver_results).Count $Golden.review.solver_result_count "measured review solver_results count"
    $reviewSolver = @($reviewSimulation.solver_results)[0]
    Assert-ArtifactValue $reviewSolver.state $Golden.review.simulation_state_name "measured review solver state"
    Assert-ArtifactNumber @($reviewSolver.points).Count $Golden.review.simulation_point_count "measured review solver point count"
    Assert-ArtifactFloat $reviewSolver.final_value $Golden.review.simulation_final_temp_deg_c "measured review solver final_value"

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "measured report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "measured report_spec.report_schema_version"
    Assert-ArtifactNumber @($reportSpec.computed_metrics).Count $Golden.report_spec.metric_count "measured report_spec.computed_metrics count"
    $reportMetric = @($reportSpec.computed_metrics)[0]
    Assert-ArtifactValue $reportMetric.binding $Golden.report_spec.metric_binding "measured report_spec metric binding"
    Assert-ArtifactValue $reportMetric.quantity_kind $Golden.report_spec.metric_quantity "measured report_spec metric quantity"
    Assert-ArtifactValue $reportMetric.unit $Golden.report_spec.metric_unit "measured report_spec metric unit"
    Assert-ArtifactFloat $reportMetric.value $Golden.report_spec.metric_value "measured report_spec metric value"
    Assert-ArtifactNumber @($reportSpec.validations).Count $Golden.report_spec.validation_count "measured report_spec.validations count"
    $reportValidation = @($reportSpec.validations)[0]
    Assert-ArtifactValue $reportValidation.expression $Golden.report_spec.validation_expression "measured report_spec validation expression"
    Assert-ArtifactValue $reportValidation.status $Golden.report_spec.validation_status "measured report_spec validation status"
    Assert-ArtifactValue $reportValidation.unit $Golden.report_spec.validation_unit "measured report_spec validation unit"
    Assert-ArtifactFloat $reportValidation.left_value $Golden.report_spec.validation_left_value "measured report_spec validation left_value"
    Assert-ArtifactFloat $reportValidation.right_value $Golden.report_spec.validation_right_value "measured report_spec validation right_value"
    Assert-ArtifactNumber @($reportSpec.time_alignments).Count $Golden.report_spec.time_alignment_count "measured report_spec.time_alignments count"
    $reportAlignment = @($reportSpec.time_alignments) | Where-Object { $_.left -eq $Golden.report_spec.alignment_left -and $_.right -eq $Golden.report_spec.alignment_right } | Select-Object -First 1
    Assert-Artifact ($null -ne $reportAlignment) "measured report_spec missing measured/sim alignment"
    Assert-ArtifactValue $reportAlignment.status $Golden.report_spec.alignment_status "measured report_spec alignment status"
    Assert-ArtifactNumber $reportAlignment.matched_count $Golden.report_spec.alignment_matched_count "measured report_spec alignment matched_count"
    $reportSolver = @(@($reportSpec.system_ir)[0].solver_results)[0]
    Assert-ArtifactValue $reportSolver.state $Golden.report_spec.solver_state_name "measured report_spec solver state"
    Assert-ArtifactValue $reportSolver.method $Golden.report_spec.solver_method "measured report_spec solver method"
    Assert-ArtifactNumber $reportSolver.step_count $Golden.report_spec.solver_step_count "measured report_spec solver step_count"
    Assert-ArtifactFloat $reportSolver.final_value $Golden.report_spec.solver_final_temp_deg_c "measured report_spec solver final_value"

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "measured result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "measured result.result_format_version"
    Assert-ArtifactValue $result.execution_profile $Golden.profile "measured result.execution_profile"
    Assert-ArtifactNumber @($result.typed_payload.metrics).Count $Golden.result.metric_count "measured result.typed_payload.metrics count"
    $resultMetric = @($result.typed_payload.metrics)[0]
    Assert-ArtifactValue $resultMetric.binding $Golden.result.metric_binding "measured result metric binding"
    Assert-ArtifactValue $resultMetric.quantity_kind $Golden.result.metric_quantity "measured result metric quantity"
    Assert-ArtifactValue $resultMetric.unit $Golden.result.metric_unit "measured result metric unit"
    Assert-ArtifactFloat $resultMetric.value $Golden.result.metric_value "measured result metric value"
    Assert-ArtifactNumber @($result.typed_payload.validations).Count $Golden.result.validation_count "measured result.typed_payload.validations count"
    $resultValidation = @($result.typed_payload.validations)[0]
    Assert-ArtifactValue $resultValidation.expression $Golden.result.validation_expression "measured result validation expression"
    Assert-ArtifactValue $resultValidation.status $Golden.result.validation_status "measured result validation status"
    Assert-ArtifactValue $resultValidation.unit $Golden.result.validation_unit "measured result validation unit"
    Assert-ArtifactFloat $resultValidation.left_value $Golden.result.validation_left_value "measured result validation left_value"
    Assert-ArtifactFloat $resultValidation.right_value $Golden.result.validation_right_value "measured result validation right_value"
    Assert-ArtifactNumber @($result.typed_payload.time_alignments).Count $Golden.result.time_alignment_count "measured result.time_alignments count"
    $resultAlignment = @($result.typed_payload.time_alignments) | Where-Object { $_.left -eq $Golden.result.alignment_left -and $_.right -eq $Golden.result.alignment_right } | Select-Object -First 1
    Assert-Artifact ($null -ne $resultAlignment) "measured result missing measured/sim alignment"
    Assert-ArtifactValue $resultAlignment.status $Golden.result.alignment_status "measured result alignment status"
    Assert-ArtifactNumber $resultAlignment.matched_count $Golden.result.alignment_matched_count "measured result alignment matched_count"
    $resultSolver = @($result.typed_payload.systems)[0].solver_result
    Assert-ArtifactValue $resultSolver.status $Golden.result.solver_status "measured result solver status"
    Assert-ArtifactValue $resultSolver.method $Golden.result.solver_method "measured result solver method"
    Assert-ArtifactNumber $resultSolver.step_count $Golden.result.solver_step_count "measured result solver step_count"
    Assert-ArtifactFloat $resultSolver.final_value $Golden.result.solver_final_temp_deg_c "measured result solver final_value"

    $plotSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\plots\plot_spec.json")
    Assert-ArtifactValue $plotSpec.format $Golden.plot_spec.format "measured plot_spec.format"
    Assert-ArtifactNumber $plotSpec.plot_spec_version $Golden.plot_spec.plot_spec_version "measured plot_spec.plot_spec_version"
    Assert-ArtifactValue $plotSpec.plot_type $Golden.plot_spec.plot_type "measured plot_spec.plot_type"
    Assert-ArtifactValue $plotSpec.title $Golden.plot_spec.title "measured plot_spec.title"
    Assert-ArtifactValue $plotSpec.x_axis.unit $Golden.plot_spec.x_unit "measured plot_spec.x_axis.unit"
    Assert-ArtifactValue $plotSpec.y_axis.unit $Golden.plot_spec.y_unit "measured plot_spec.y_axis.unit"
    Assert-ArtifactNumber @($plotSpec.series).Count $Golden.plot_spec.series_count "measured plot_spec.series count"
    $measuredSeries = @($plotSpec.series)[0]
    $simSeries = @($plotSpec.series)[1]
    Assert-ArtifactValue $measuredSeries.name $Golden.plot_spec.first_series "measured plot_spec first series"
    Assert-ArtifactValue $simSeries.name $Golden.plot_spec.second_series "measured plot_spec second series"
    Assert-ArtifactNumber @($measuredSeries.points).Count $Golden.plot_spec.point_count "measured plot_spec measured point count"
    Assert-ArtifactNumber @($simSeries.points).Count $Golden.plot_spec.point_count "measured plot_spec simulated point count"
    Assert-ArtifactFloat @(@($measuredSeries.points)[0])[1] $Golden.plot_spec.first_measured_y_deg_c "measured plot_spec first measured y"
    Assert-ArtifactFloat @(@($simSeries.points)[@($simSeries.points).Count - 1])[1] $Golden.plot_spec.last_simulated_y_deg_c "measured plot_spec last simulated y"
}

function Assert-MultiStateThermalGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--save-artifacts"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "multi-state review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "multi-state review.review_schema_version"
    Assert-ArtifactValue (Get-NormalizedArtifactPath $review.source_path) (Get-NormalizedArtifactPath $Golden.source) "multi-state review.source_path"
    Assert-ArtifactNumber @($review.simulation_results).Count $Golden.review.simulation_result_count "multi-state review.simulation_results count"
    $reviewSimulation = @($review.simulation_results)[0]
    Assert-ArtifactValue $reviewSimulation.method $Golden.review.simulation_method "multi-state review simulation method"
    Assert-ArtifactNumber @($reviewSimulation.solver_results).Count $Golden.review.solver_result_count "multi-state review solver_results count"
    foreach ($expectedState in @($Golden.review.states)) {
        $solver = @($reviewSimulation.solver_results) | Where-Object { $_.state -eq $expectedState.name } | Select-Object -First 1
        Assert-Artifact ($null -ne $solver) "multi-state review missing solver state $($expectedState.name)"
        Assert-ArtifactValue $solver.status $expectedState.status "multi-state review $($expectedState.name) status"
        Assert-ArtifactValue $solver.method $Golden.review.simulation_method "multi-state review $($expectedState.name) method"
        Assert-ArtifactNumber $solver.step_count $expectedState.step_count "multi-state review $($expectedState.name) step_count"
        Assert-ArtifactNumber @($solver.points).Count $expectedState.point_count "multi-state review $($expectedState.name) point count"
        Assert-ArtifactFloat $solver.final_value $expectedState.final_value "multi-state review $($expectedState.name) final_value"
    }

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "multi-state report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "multi-state report_spec.report_schema_version"
    Assert-ArtifactNumber @($reportSpec.state_space_vectors).Count $Golden.report_spec.state_space_vector_count "multi-state report_spec.state_space_vectors count"
    Assert-ArtifactNumber @($reportSpec.linear_operators).Count $Golden.report_spec.linear_operator_count "multi-state report_spec.linear_operators count"
    Assert-ArtifactNumber @(@($reportSpec.system_ir)[0].solver_results).Count $Golden.report_spec.solver_result_count "multi-state report_spec solver_results count"
    foreach ($expectedState in @($Golden.report_spec.states)) {
        $solver = @(@($reportSpec.system_ir)[0].solver_results) | Where-Object { $_.state -eq $expectedState.name } | Select-Object -First 1
        Assert-Artifact ($null -ne $solver) "multi-state report_spec missing solver state $($expectedState.name)"
        Assert-ArtifactValue $solver.status $expectedState.status "multi-state report_spec $($expectedState.name) status"
        Assert-ArtifactValue $solver.method $Golden.report_spec.solver_method "multi-state report_spec $($expectedState.name) method"
        Assert-ArtifactNumber $solver.step_count $expectedState.step_count "multi-state report_spec $($expectedState.name) step_count"
        Assert-ArtifactFloat $solver.final_value $expectedState.final_value "multi-state report_spec $($expectedState.name) final_value"
    }

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "multi-state result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "multi-state result.result_format_version"
    Assert-ArtifactNumber @($result.typed_payload.systems).Count $Golden.result.system_count "multi-state result systems count"
    $resultSystem = @($result.typed_payload.systems)[0]
    Assert-ArtifactNumber @($resultSystem.solver_results).Count $Golden.result.solver_result_count "multi-state result solver_results count"
    foreach ($expectedState in @($Golden.result.states)) {
        $solver = @($resultSystem.solver_results) | Where-Object { $_.state -eq $expectedState.name } | Select-Object -First 1
        Assert-Artifact ($null -ne $solver) "multi-state result missing solver state $($expectedState.name)"
        Assert-ArtifactValue $solver.status $expectedState.status "multi-state result $($expectedState.name) status"
        Assert-ArtifactValue $solver.method $Golden.result.solver_method "multi-state result $($expectedState.name) method"
        Assert-ArtifactNumber $solver.step_count $expectedState.step_count "multi-state result $($expectedState.name) step_count"
        Assert-ArtifactNumber @($solver.points).Count $expectedState.point_count "multi-state result $($expectedState.name) point count"
        Assert-ArtifactFloat $solver.final_value $expectedState.final_value "multi-state result $($expectedState.name) final_value"
    }

    $plotSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\plots\plot_spec.json")
    Assert-ArtifactValue $plotSpec.format $Golden.plot_spec.format "multi-state plot_spec.format"
    Assert-ArtifactNumber $plotSpec.plot_spec_version $Golden.plot_spec.plot_spec_version "multi-state plot_spec.plot_spec_version"
    Assert-ArtifactNumber @($plotSpec.series).Count $Golden.plot_spec.series_count "multi-state plot_spec.series count"
    foreach ($expectedSeries in @($Golden.plot_spec.series)) {
        $series = @($plotSpec.series) | Where-Object { $_.name -eq $expectedSeries.name } | Select-Object -First 1
        Assert-Artifact ($null -ne $series) "multi-state plot_spec missing series $($expectedSeries.name)"
        Assert-ArtifactNumber @($series.points).Count $expectedSeries.point_count "multi-state plot_spec $($expectedSeries.name) point count"
    }
}

function Assert-ComponentSolverGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--save-artifacts"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "$($Golden.name) review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "$($Golden.name) review.review_schema_version"
    Assert-ArtifactValue (Get-NormalizedArtifactPath $review.source_path) (Get-NormalizedArtifactPath $Golden.source) "$($Golden.name) review.source_path"
    Assert-ArtifactNumber @($review.domain_summary).Count $Golden.review.domain_count "$($Golden.name) review domain count"
    Assert-ArtifactNumber @($review.component_summary).Count $Golden.review.component_count "$($Golden.name) review component count"
    Assert-ArtifactNumber @($review.connection_summary).Count $Golden.review.connection_count "$($Golden.name) review connection count"
    Assert-ArtifactNumber @($review.assembly_summary).Count $Golden.review.assembly_count "$($Golden.name) review assembly count"
    $reviewAssembly = @($review.assembly_summary)[0]
    Assert-ArtifactNumber @($reviewAssembly.domain_plans).Count $Golden.review.domain_plan_count "$($Golden.name) review domain plan count"
    Assert-ArtifactNumber @($reviewAssembly.equations).Count $Golden.review.equation_count "$($Golden.name) review equation count"
    Assert-ArtifactNumber @($reviewAssembly.variables).Count $Golden.review.unknown_count "$($Golden.name) review variable count"
    Assert-ArtifactNumber $reviewAssembly.boundary.equation_count $Golden.review.equation_count "$($Golden.name) review boundary equation count"
    Assert-ArtifactNumber $reviewAssembly.boundary.unknown_count $Golden.review.unknown_count "$($Golden.name) review boundary unknown count"
    Assert-ArtifactNumber @($reviewAssembly.residual_graph.dependencies).Count $Golden.review.residual_dependency_count "$($Golden.name) review residual dependency count"

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "$($Golden.name) report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "$($Golden.name) report_spec.report_schema_version"
    Assert-ArtifactNumber @($reportSpec.domain_summary).Count $Golden.report_spec.domain_count "$($Golden.name) report_spec domain count"
    Assert-ArtifactNumber @($reportSpec.component_summary).Count $Golden.report_spec.component_count "$($Golden.name) report_spec component count"
    Assert-ArtifactNumber @($reportSpec.assembly_summary).Count $Golden.report_spec.assembly_count "$($Golden.name) report_spec assembly count"
    $reportAssembly = @($reportSpec.assembly_summary)[0]
    Assert-ArtifactValue $reportAssembly.solver_result.status $Golden.report_spec.solver_status "$($Golden.name) report_spec solver status"
    Assert-ArtifactValue $reportAssembly.solver_result.method $Golden.report_spec.solver_method "$($Golden.name) report_spec solver method"
    Assert-ArtifactValue $reportAssembly.solver_result.convergence_status $Golden.report_spec.solver_convergence_status "$($Golden.name) report_spec solver convergence"
    Assert-ArtifactNumber @($reportAssembly.solver_result.variables).Count $Golden.report_spec.solver_variable_count "$($Golden.name) report_spec solver variable count"
    Assert-ArtifactNumber @($reportAssembly.solver_result.largest_residuals).Count $Golden.report_spec.largest_residual_count "$($Golden.name) report_spec largest residual count"

    $result = Read-ArtifactJson (Join-Path $RepoRoot "build\result\result.engres")
    Assert-ArtifactValue $result.format $Golden.result.format "$($Golden.name) result.format"
    Assert-ArtifactNumber $result.result_format_version $Golden.result.result_format_version "$($Golden.name) result.result_format_version"
    Assert-ArtifactNumber @($result.typed_payload.component_solutions).Count $Golden.result.component_solution_count "$($Golden.name) result component solution count"
    $solution = @($result.typed_payload.component_solutions)[0]
    Assert-ArtifactValue $solution.status $Golden.result.status "$($Golden.name) result solver status"
    Assert-ArtifactValue $solution.method $Golden.result.method "$($Golden.name) result solver method"
    Assert-ArtifactValue $solution.convergence_status $Golden.result.convergence_status "$($Golden.name) result convergence"
    Assert-ArtifactNumber $solution.equation_count $Golden.result.equation_count "$($Golden.name) result equation count"
    Assert-ArtifactNumber $solution.unknown_count $Golden.result.unknown_count "$($Golden.name) result unknown count"
    Assert-ArtifactNumber @($solution.variables).Count $Golden.result.variable_count "$($Golden.name) result variable count"
    Assert-ArtifactNumber @($solution.largest_residuals).Count $Golden.result.largest_residual_count "$($Golden.name) result largest residual count"
    Assert-ArtifactFloat $solution.residual_norm $Golden.result.residual_norm "$($Golden.name) result residual_norm"
    foreach ($expectedVariable in @($Golden.result.expected_variables)) {
        $variable = @($solution.variables) | Where-Object { $_.name -eq $expectedVariable.name } | Select-Object -First 1
        Assert-Artifact ($null -ne $variable) "$($Golden.name) result missing solved variable $($expectedVariable.name)"
        Assert-ArtifactFloat $variable.value $expectedVariable.value "$($Golden.name) result $($expectedVariable.name)"
    }
}

function Assert-BehaviorNodesGolden {
    param(
        [Parameter(Mandatory = $true)]
        $Golden,

        [Parameter(Mandatory = $true)]
        [string] $Eng
    )

    Remove-Item -LiteralPath (Join-Path $RepoRoot "build\result") -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-Native $Eng "run" $Golden.source "--save-artifacts"

    $review = Read-ArtifactJson (Join-Path $RepoRoot "build\result\review.json")
    Assert-ArtifactValue $review.format $Golden.review.format "$($Golden.name) review.format"
    Assert-ArtifactNumber $review.review_schema_version $Golden.review.review_schema_version "$($Golden.name) review.review_schema_version"
    Assert-ArtifactValue (Get-NormalizedArtifactPath $review.source_path) (Get-NormalizedArtifactPath $Golden.source) "$($Golden.name) review.source_path"
    Assert-ArtifactNumber @($review.domain_summary).Count $Golden.review.domain_count "$($Golden.name) review domain count"
    Assert-ArtifactNumber @($review.component_summary).Count $Golden.review.component_count "$($Golden.name) review component count"
    Assert-ArtifactNumber @($review.connection_summary).Count $Golden.review.connection_count "$($Golden.name) review connection count"
    Assert-ArtifactNumber @($review.assembly_summary).Count $Golden.review.assembly_count "$($Golden.name) review assembly count"
    Assert-ArtifactNumber @($review.component_graph.behavior_nodes).Count $Golden.review.behavior_node_count "$($Golden.name) review behavior node count"
    foreach ($expectedNode in @($Golden.behavior_nodes)) {
        $node = @($review.component_graph.behavior_nodes) | Where-Object { $_.behavior_kind -eq $expectedNode.behavior_kind } | Select-Object -First 1
        Assert-Artifact ($null -ne $node) "$($Golden.name) review missing behavior node $($expectedNode.behavior_kind)"
        Assert-ArtifactValue $node.status $expectedNode.status "$($Golden.name) review $($expectedNode.behavior_kind) status"
        Assert-ArtifactValue $node.signal $expectedNode.signal "$($Golden.name) review $($expectedNode.behavior_kind) signal"
        Assert-ArtifactNullableValue $node.contract_status $expectedNode.contract_status "$($Golden.name) review $($expectedNode.behavior_kind) contract_status"
        Assert-ArtifactNullableValue $node.jacobian_policy $expectedNode.jacobian_policy "$($Golden.name) review $($expectedNode.behavior_kind) jacobian_policy"
        Assert-ArtifactNullableValue $node.profile_policy $expectedNode.profile_policy "$($Golden.name) review $($expectedNode.behavior_kind) profile_policy"
        if ($null -ne $expectedNode.delay_s) {
            Assert-ArtifactFloat $node.delay_s $expectedNode.delay_s "$($Golden.name) review $($expectedNode.behavior_kind) delay_s"
        }
    }

    $reportSpec = Read-ArtifactJson (Join-Path $RepoRoot "build\result\report_spec.json")
    Assert-ArtifactValue $reportSpec.format $Golden.report_spec.format "$($Golden.name) report_spec.format"
    Assert-ArtifactNumber $reportSpec.report_schema_version $Golden.report_spec.report_schema_version "$($Golden.name) report_spec.report_schema_version"
    Assert-ArtifactNumber @($reportSpec.component_graph.behavior_nodes).Count $Golden.report_spec.behavior_node_count "$($Golden.name) report_spec behavior node count"
    $reportAssembly = @($reportSpec.assembly_summary)[0]
    Assert-ArtifactValue $reportAssembly.solver_preview.delay_history $Golden.report_spec.delay_history "$($Golden.name) report_spec delay history"
    Assert-ArtifactValue $reportAssembly.solver_preview.predictor $Golden.report_spec.predictor "$($Golden.name) report_spec predictor"
    Assert-ArtifactValue $reportAssembly.solver_preview.external_adapter $Golden.report_spec.external_adapter "$($Golden.name) report_spec external adapter"
    foreach ($expectedNode in @($Golden.behavior_nodes)) {
        $node = @($reportSpec.component_graph.behavior_nodes) | Where-Object { $_.behavior_kind -eq $expectedNode.behavior_kind } | Select-Object -First 1
        Assert-Artifact ($null -ne $node) "$($Golden.name) report_spec missing behavior node $($expectedNode.behavior_kind)"
        Assert-ArtifactValue $node.status $expectedNode.status "$($Golden.name) report_spec $($expectedNode.behavior_kind) status"
        Assert-ArtifactValue $node.signal $expectedNode.signal "$($Golden.name) report_spec $($expectedNode.behavior_kind) signal"
        Assert-ArtifactNullableValue $node.contract_status $expectedNode.contract_status "$($Golden.name) report_spec $($expectedNode.behavior_kind) contract_status"
        Assert-ArtifactNullableValue $node.jacobian_policy $expectedNode.jacobian_policy "$($Golden.name) report_spec $($expectedNode.behavior_kind) jacobian_policy"
        Assert-ArtifactNullableValue $node.profile_policy $expectedNode.profile_policy "$($Golden.name) report_spec $($expectedNode.behavior_kind) profile_policy"
        if ($null -ne $expectedNode.delay_s) {
            Assert-ArtifactFloat $node.delay_s $expectedNode.delay_s "$($Golden.name) report_spec $($expectedNode.behavior_kind) delay_s"
        }
        $inputContract = @($node.contract_inputs) | Select-Object -First 1
        Assert-Artifact ($null -ne $inputContract) "$($Golden.name) report_spec $($expectedNode.behavior_kind) input contract"
        Assert-ArtifactValue $inputContract.status $expectedNode.input_contract_status "$($Golden.name) report_spec $($expectedNode.behavior_kind) input contract status"
        Assert-ArtifactValue $inputContract.quantity_kind $expectedNode.input_quantity_kind "$($Golden.name) report_spec $($expectedNode.behavior_kind) input quantity"
    }
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
    $systemGolden = Read-ArtifactJson (Join-Path $goldenRoot "internal_02_simple_system.golden.json")
    $measuredGolden = Read-ArtifactJson (Join-Path $goldenRoot "internal_17_measured_vs_simulated.golden.json")
    $multiStateGolden = Read-ArtifactJson (Join-Path $goldenRoot "internal_20_multi_state_thermal.golden.json")
    $thermalAssemblyGolden = Read-ArtifactJson (Join-Path $goldenRoot "internal_21_thermal_component_assembly.golden.json")
    $multiDomainGolden = Read-ArtifactJson (Join-Path $goldenRoot "internal_22_multi_domain_boundary_solve.golden.json")
    $behaviorNodesGolden = Read-ArtifactJson (Join-Path $goldenRoot "internal_25_component_behavior_nodes.golden.json")

    Assert-CsvPlotGolden $csvGolden $Eng
    Assert-SystemGolden $systemGolden $Eng
    Assert-MeasuredVsSimulatedGolden $measuredGolden $Eng
    Assert-MultiStateThermalGolden $multiStateGolden $Eng
    Assert-ComponentSolverGolden $thermalAssemblyGolden $Eng
    Assert-ComponentSolverGolden $multiDomainGolden $Eng
    Assert-BehaviorNodesGolden $behaviorNodesGolden $Eng

    Write-Host "Artifact check passed. Validated schema files and internal system artifact fixtures."
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

function Invoke-VscodeBuildGrammar {
    Set-DevEnvironment
    & (Join-Path $RepoRoot "tools\vscode-englang\scripts\build-grammar.ps1") -ExtensionRoot (Join-Path $RepoRoot "tools\vscode-englang")
}

function Invoke-VscodeBuildEditorMetadata {
    Set-DevEnvironment
    & (Join-Path $RepoRoot "tools\vscode-englang\scripts\build-editor-metadata.ps1") -ExtensionRoot (Join-Path $RepoRoot "tools\vscode-englang")
}

function Invoke-VscodeGrammarTest {
    Set-DevEnvironment
    & (Join-Path $RepoRoot "tools\vscode-englang\scripts\test-grammar.ps1") -ExtensionRoot (Join-Path $RepoRoot "tools\vscode-englang")
}

function Read-JavaScriptStringArrayConst {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][string] $Name
    )

    $pattern = "const\s+$([regex]::Escape($Name))\s*=\s*\[(?<body>.*?)\];"
    $match = [regex]::Match($Source, $pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
    if (-not $match.Success) {
        throw "missing JavaScript string array constant $Name"
    }
    return @([regex]::Matches($match.Groups["body"].Value, '"([^"]+)"') | ForEach-Object { $_.Groups[1].Value })
}

function Read-RustStringSliceConst {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][string] $Name
    )

    $pattern = "pub\s+const\s+$([regex]::Escape($Name))\s*:\s*&\[\&str\]\s*=\s*&\[(?<body>.*?)\];"
    $match = [regex]::Match($Source, $pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
    if (-not $match.Success) {
        throw "missing Rust string slice constant $Name"
    }
    return @([regex]::Matches($match.Groups["body"].Value, '"([^"]+)"') | ForEach-Object { $_.Groups[1].Value })
}

function Assert-SameStringSequence {
    param(
        [Parameter(Mandatory = $true)][string[]] $Left,
        [Parameter(Mandatory = $true)][string[]] $Right,
        [Parameter(Mandatory = $true)][string] $Description
    )

    if ($Left.Count -ne $Right.Count) {
        throw "${Description} length mismatch: $($Left.Count) != $($Right.Count)"
    }
    for ($index = 0; $index -lt $Left.Count; $index++) {
        if ($Left[$index] -ne $Right[$index]) {
            throw "${Description} mismatch at index ${index}: $($Left[$index]) != $($Right[$index])"
        }
    }
}

function Assert-VscodeExtensionContract {
    $ExtensionRoot = Join-Path $RepoRoot "tools\vscode-englang"
    $PackageJsonPath = Join-Path $ExtensionRoot "package.json"
    $ExtensionJsPath = Join-Path $ExtensionRoot "extension.js"
    $CompletionProviderPath = Join-Path $ExtensionRoot "completionProvider.js"
    $LocalCodeActionsPath = Join-Path $ExtensionRoot "localCodeActions.js"
    $LspCodeActionsPath = Join-Path $ExtensionRoot "lspCodeActions.js"
    $LspKindsPath = Join-Path $ExtensionRoot "lspKinds.js"
    $LspNavigationPath = Join-Path $ExtensionRoot "lspNavigation.js"
    $LspRangesPath = Join-Path $ExtensionRoot "lspRanges.js"
    $LspSemanticTokensPath = Join-Path $ExtensionRoot "lspSemanticTokens.js"
    $ArtifactRegistryPath = Join-Path $ExtensionRoot "artifactRegistry.js"
    $EditorMetadataLoaderPath = Join-Path $ExtensionRoot "editorMetadata.js"
    $ExecutionProfilesPath = Join-Path $ExtensionRoot "executionProfiles.js"
    $ModuleStatusPath = Join-Path $ExtensionRoot "moduleStatus.js"
    $SnippetsPath = Join-Path $ExtensionRoot "snippets\eng.json"
    $LspSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\lib.rs"
    $LspCliSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\main.rs"
    $EditorMetadataPath = Join-Path $ExtensionRoot "generated\editor\englang-editor-metadata.json"
    $SemanticLegendPath = Join-Path $ExtensionRoot "generated\editor\englang-semantic-legend.json"
    $CompletionsPath = Join-Path $ExtensionRoot "generated\editor\englang-completions.json"
    $TokenScopesDocPath = Join-Path $RepoRoot "docs\internal\editor\token_scopes.md"
    $DevScriptPath = Join-Path $RepoRoot "scripts\dev.ps1"
    $VscodeReadmePath = Join-Path $ExtensionRoot "README.md"
    $NativeIdeHowtoPath = Join-Path $RepoRoot "docs\user\howto\use_native_ide.md"
    $UserGuidePath = Join-Path $RepoRoot "docs\user\user_guide.md"
    $FeatureMaturityPath = Join-Path $RepoRoot "docs\current\feature_maturity_matrix.md"
    $MainInternalStatusPath = Join-Path $RepoRoot "docs\current\main_internal_status.md"
    $CurrentStatusPath = Join-Path $RepoRoot "docs\current\status.md"
    $CurrentTracksPath = Join-Path $RepoRoot "docs\current\tracks.md"

    if (-not (Test-Path $PackageJsonPath)) {
        throw "missing VS Code extension package.json at $PackageJsonPath"
    }
    if (-not (Test-Path $ExtensionJsPath)) {
        throw "missing VS Code extension entrypoint at $ExtensionJsPath"
    }
    if (-not (Test-Path $CompletionProviderPath)) {
        throw "missing VS Code completion provider at $CompletionProviderPath"
    }
    if (-not (Test-Path $LocalCodeActionsPath)) {
        throw "missing VS Code local quick fix provider at $LocalCodeActionsPath"
    }
    if (-not (Test-Path $LspCodeActionsPath)) {
        throw "missing VS Code LSP code action bridge at $LspCodeActionsPath"
    }
    if (-not (Test-Path $LspKindsPath)) {
        throw "missing VS Code LSP kind bridge at $LspKindsPath"
    }
    if (-not (Test-Path $LspNavigationPath)) {
        throw "missing VS Code LSP navigation bridge at $LspNavigationPath"
    }
    if (-not (Test-Path $LspRangesPath)) {
        throw "missing VS Code LSP range bridge at $LspRangesPath"
    }
    if (-not (Test-Path $LspSemanticTokensPath)) {
        throw "missing VS Code LSP semantic token bridge at $LspSemanticTokensPath"
    }
    if (-not (Test-Path $ArtifactRegistryPath)) {
        throw "missing VS Code artifact registry at $ArtifactRegistryPath"
    }
    if (-not (Test-Path $EditorMetadataLoaderPath)) {
        throw "missing VS Code editor metadata loader at $EditorMetadataLoaderPath"
    }
    if (-not (Test-Path $ExecutionProfilesPath)) {
        throw "missing VS Code execution profiles registry at $ExecutionProfilesPath"
    }
    if (-not (Test-Path $ModuleStatusPath)) {
        throw "missing VS Code module status wording registry at $ModuleStatusPath"
    }
    if (-not (Test-Path $SnippetsPath)) {
        throw "missing VS Code snippets at $SnippetsPath"
    }
    if (-not (Test-Path $LspSourcePath)) {
        throw "missing eng_lsp source at $LspSourcePath"
    }
    if (-not (Test-Path $LspCliSourcePath)) {
        throw "missing eng_lsp CLI source at $LspCliSourcePath"
    }
    foreach ($RequiredMetadataPath in @($EditorMetadataPath, $SemanticLegendPath, $CompletionsPath)) {
        if (-not (Test-Path $RequiredMetadataPath)) {
            throw "missing generated VS Code editor metadata at $RequiredMetadataPath"
        }
    }
    if (-not (Test-Path $TokenScopesDocPath)) {
        throw "missing editor token scope contract at $TokenScopesDocPath"
    }
    foreach ($RequiredDocPath in @($DevScriptPath, $VscodeReadmePath, $NativeIdeHowtoPath, $UserGuidePath, $FeatureMaturityPath, $MainInternalStatusPath, $CurrentStatusPath, $CurrentTracksPath)) {
        if (-not (Test-Path $RequiredDocPath)) {
            throw "missing VS Code install contract input at $RequiredDocPath"
        }
    }

    $PackageSource = Get-Content -LiteralPath $PackageJsonPath -Raw
    $Package = $PackageSource | ConvertFrom-Json
    $TokenScopesDoc = Get-Content -LiteralPath $TokenScopesDocPath -Raw
    $DevScriptSource = Get-Content -LiteralPath $DevScriptPath -Raw
    $VscodeReadmeSource = Get-Content -LiteralPath $VscodeReadmePath -Raw
    $NativeIdeHowtoSource = Get-Content -LiteralPath $NativeIdeHowtoPath -Raw
    $UserGuideSource = Get-Content -LiteralPath $UserGuidePath -Raw
    $FeatureMaturitySource = Get-Content -LiteralPath $FeatureMaturityPath -Raw
    $MainInternalStatusSource = Get-Content -LiteralPath $MainInternalStatusPath -Raw
    $CurrentStatusSource = Get-Content -LiteralPath $CurrentStatusPath -Raw
    $CurrentTracksSource = Get-Content -LiteralPath $CurrentTracksPath -Raw
    if ($Package.name -ne "englang") {
        throw "VS Code extension package name must be englang"
    }
    if ($Package.main -ne "./extension.js") {
        throw "VS Code extension main must be ./extension.js"
    }
    $ActivationEvents = @($Package.activationEvents)
    foreach ($RequiredActivationEvent in @("onLanguage:englang", "workspaceContains:**/*.eng")) {
        if ($ActivationEvents -notcontains $RequiredActivationEvent) {
            throw "VS Code extension activationEvents missing $RequiredActivationEvent"
        }
    }
    $Language = $Package.contributes.languages | Select-Object -First 1
    if ($Language.id -ne "englang") {
        throw "VS Code extension must contribute englang language id"
    }
    if ($Language.extensions -notcontains ".eng") {
        throw "VS Code extension must register .eng files"
    }
    $Grammar = $Package.contributes.grammars | Select-Object -First 1
    if ($Grammar.language -ne "englang") {
        throw "VS Code extension grammar must target englang language id"
    }
    $GrammarPath = Join-Path $ExtensionRoot $Grammar.path
    if (-not (Test-Path $GrammarPath)) {
        throw "VS Code extension missing grammar at $GrammarPath"
    }
    $GrammarSourcePath = Join-Path $ExtensionRoot "syntaxes\eng.tmLanguage.source.json"
    if (-not (Test-Path $GrammarSourcePath)) {
        throw "VS Code extension missing source grammar at $GrammarSourcePath"
    }
    $LanguageConfigurationPath = Join-Path $ExtensionRoot "language-configuration.json"
    if (-not (Test-Path $LanguageConfigurationPath)) {
        throw "VS Code extension missing language configuration at $LanguageConfigurationPath"
    }
    $LanguageConfiguration = Get-Content -LiteralPath $LanguageConfigurationPath -Raw | ConvertFrom-Json
    if ($LanguageConfiguration.comments.lineComment -ne "#") {
        throw "VS Code extension language configuration must keep # as line comment"
    }
    if ($LanguageConfiguration.indentationRules.increaseIndentPattern -ne '^.*\{\s*(#.*)?$') {
        throw "VS Code extension language configuration must indent after block openers"
    }
    if ($LanguageConfiguration.indentationRules.decreaseIndentPattern -ne '^\s*\}') {
        throw "VS Code extension language configuration must outdent block closers"
    }
    if ($LanguageConfiguration.wordPattern -ne '(-?\d+(?:\.\d+)?)|([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)') {
        throw "VS Code extension language configuration must treat dotted EngLang symbols as words"
    }
    $Snippets = Get-Content -LiteralPath $SnippetsPath -Raw | ConvertFrom-Json
    foreach ($RequiredSnippet in @(
        @{ Name = "Native HTTP GET"; Tokens = @("http get", "fixture", "expected_sha256", "cache_key") },
        @{ Name = "Native HTTP POST body"; Tokens = @("http post", "body =", "expected_sha256", "cache_key") },
        @{ Name = "Sample LHS table"; Tokens = @("sample lhs", "count =", "seed =", "uniform(") },
        @{ Name = "Apply case template"; Tokens = @("apply", "over", "template = file", "{case_dir}") },
        @{ Name = "Regression prediction table"; Tokens = @("regression_table", "model_card", "evaluate", "predict") },
        @{ Name = "SQLite table write"; Tokens = @("open sqlite", ".table", "transaction = commit") },
        @{ Name = "Standard text artifact"; Tokens = @("write standard_text", "output = join", "overwrite = true") },
        @{ Name = "Plot line"; Tokens = @("plot", "unit y =", "title =") }
    )) {
        $SnippetProperty = $Snippets.PSObject.Properties[$RequiredSnippet.Name]
        if ($null -eq $SnippetProperty) {
            throw "VS Code snippets missing native snippet $($RequiredSnippet.Name)"
        }
        $SnippetBody = (@($SnippetProperty.Value.body) -join "`n")
        foreach ($RequiredSnippetToken in $RequiredSnippet.Tokens) {
            if (-not $SnippetBody.Contains($RequiredSnippetToken)) {
                throw "VS Code snippet $($RequiredSnippet.Name) missing token $RequiredSnippetToken"
            }
        }
    }
    foreach ($RequiredVscodeInstallPattern in @(
        '(?m)^\s+"vscode-package"\s*\{\s*Invoke-VscodePackage\s*\}',
        '(?m)^\s+"vscode-install"\s*\{\s*Invoke-VscodeInstall\s*\}',
        '(?m)^\s+\.\\dev\.bat vscode-package Build a local installable VS Code extension VSIX\s*$',
        '(?m)^\s+\.\\dev\.bat vscode-install Build and install the EngLang VS Code extension with the code CLI\s*$'
    )) {
        if ($DevScriptSource -notmatch $RequiredVscodeInstallPattern) {
            throw "dev wrapper missing VS Code local install contract pattern $RequiredVscodeInstallPattern"
        }
    }
    foreach ($RequiredVscodeInstallDocToken in @(
        ".\dev.bat vscode-install",
        ".\dev.bat vscode-package",
        "dist\local-vscode\tools\englang-vscode-<version>.vsix",
        "Extensions: Install from VSIX..."
    )) {
        if (-not $VscodeReadmeSource.Contains($RequiredVscodeInstallDocToken)) {
            throw "VS Code README missing local install token $RequiredVscodeInstallDocToken"
        }
        if (-not $NativeIdeHowtoSource.Contains($RequiredVscodeInstallDocToken)) {
            throw "native IDE how-to missing VS Code install token $RequiredVscodeInstallDocToken"
        }
    }
    $DocCommentEnterRule = @($LanguageConfiguration.onEnterRules) | Where-Object {
        $_.beforeText -eq "^\s*///.*$" -and $_.action.appendText -eq "/// "
    } | Select-Object -First 1
    if ($null -eq $DocCommentEnterRule) {
        throw "VS Code extension language configuration must continue /// doc comments on Enter"
    }
    $GrammarSource = Get-Content -LiteralPath $GrammarPath -Raw
    foreach ($RequiredGrammarToken in @(
        "read", "json", "toml", "render", "template", "open", "sqlite",
        "post", "check", "coverage", "sample", "lhs", "uniform",
        "require_one", "regression_table", "support.namespace.module.englang",
        "materialize", "apply", "collect", "case_id", "output_root", "resume", "step",
        "run_case", "train_test_split", "regression", "predict", "model_card",
        "CsvFile", "JsonFile", "DirectoryPath", "DimensionlessNumber",
        "expected_outputs", "artifact_kind", "cache_key", "allow_failure",
        "OutputManifest", "metadata_ready", "backend", "display_unit",
        "variable_scale", "consistency_tolerance", "meta.workflow.with-block.englang",
        "variable.parameter.function.englang"
    )) {
        if (-not $GrammarSource.Contains($RequiredGrammarToken)) {
            throw "VS Code grammar missing token $RequiredGrammarToken"
        }
    }
    $WorkflowPhraseScopes = [regex]::Matches(
        $GrammarSource,
        '"name"\s*:\s*"(?<scope>meta\.workflow\.[^"]+\.englang)"'
    ) | ForEach-Object { $_.Groups["scope"].Value } | Sort-Object -Unique
    foreach ($WorkflowPhraseScope in $WorkflowPhraseScopes) {
        if (-not $TokenScopesDoc.Contains($WorkflowPhraseScope)) {
            throw "editor token scope contract missing workflow phrase scope $WorkflowPhraseScope"
        }
    }
    & (Join-Path $ExtensionRoot "scripts\build-grammar.ps1") -ExtensionRoot $ExtensionRoot -Check
    & (Join-Path $ExtensionRoot "scripts\test-grammar.ps1") -ExtensionRoot $ExtensionRoot
    & (Join-Path $ExtensionRoot "scripts\build-editor-metadata.ps1") -ExtensionRoot $ExtensionRoot -Check
    $Commands = @($Package.contributes.commands | ForEach-Object { $_.command })
    foreach ($Required in @(
        "englang.checkFile",
        "englang.runFile",
        "englang.runExample",
        "englang.switchProfile",
        "englang.reviewFile",
        "englang.openReviewPanel",
        "englang.openReport",
        "englang.openLastArtifact",
        "englang.openGeneratedOutput",
        "englang.openReviewJson",
        "englang.openResultArtifact",
        "englang.openReportSpec",
        "englang.openOutputManifest",
        "englang.openRunLog",
        "englang.openStaticRunPlan",
        "englang.openRunPlan",
        "englang.openRunLock",
        "englang.openProcessResults",
        "englang.openCacheManifest",
        "englang.openTestResults",
        "englang.openPlotSpec",
        "englang.openPlotManifest",
        "englang.openPlotSvg",
        "englang.showSemanticTokensDebug"
    )) {
        if ($Commands -notcontains $Required) {
            throw "VS Code extension missing command $Required"
        }
    }
    $CommandTitles = @{}
    foreach ($Command in @($Package.contributes.commands)) {
        $CommandTitles[[string]$Command.command] = [string]$Command.title
    }
    foreach ($RequiredTitle in @(
        @{ Command = "englang.reviewFile"; Text = "Current File Review Data" },
        @{ Command = "englang.openGeneratedOutput"; Text = "Last Generated Output" },
        @{ Command = "englang.openReviewJson"; Text = "Last Run Review Data" },
        @{ Command = "englang.openResultArtifact"; Text = "Last Run Result Data" },
        @{ Command = "englang.openReportSpec"; Text = "Last Run Report Data" },
        @{ Command = "englang.openOutputManifest"; Text = "Last Run Output List" },
        @{ Command = "englang.openRunLog"; Text = "Last Run Log" },
        @{ Command = "englang.openStaticRunPlan"; Text = "Last Static Run Graph" },
        @{ Command = "englang.openRunPlan"; Text = "Last Run Graph" },
        @{ Command = "englang.openRunLock"; Text = "Last Run Reproducibility Lock" },
        @{ Command = "englang.openProcessResults"; Text = "Last Run External Process Results" },
        @{ Command = "englang.openCacheManifest"; Text = "Last Run Cache Records" },
        @{ Command = "englang.openTestResults"; Text = "Last Run Test Results" },
        @{ Command = "englang.openPlotSpec"; Text = "Last Run Plot Data" },
        @{ Command = "englang.openPlotManifest"; Text = "Last Run Plot Output List" },
        @{ Command = "englang.openPlotSvg"; Text = "Last Run Plot SVG" },
        @{ Command = "englang.showSemanticTokensDebug"; Text = "Inspect Highlight Tokens" }
    )) {
        $Title = $CommandTitles[$RequiredTitle.Command]
        if ([string]::IsNullOrWhiteSpace($Title) -or -not $Title.Contains($RequiredTitle.Text)) {
            throw "VS Code command $($RequiredTitle.Command) must expose clearer title text containing '$($RequiredTitle.Text)'"
        }
    }
    $Properties = $Package.contributes.configuration.properties
    foreach ($RequiredProperty in @("englang.runtimePath", "englang.lspPath", "englang.problemsSource", "englang.executionProfile", "englang.lintOnSave", "englang.lintOnChange", "englang.semanticHighlighting.enabled", "englang.reviewRiskDecorations.enabled")) {
        if ($null -eq $Properties.$RequiredProperty) {
            throw "VS Code extension missing configuration property $RequiredProperty"
        }
    }
    $ProblemsSourceDescription = [string]$Properties."englang.problemsSource".description
    if ($ProblemsSourceDescription -match "eng-cli|lsp-snapshot|snapshot path|metadata") {
        throw "VS Code problemsSource description must use user-facing wording, not implementation details"
    }
    $ProblemsSourceEnumDescriptions = @($Properties."englang.problemsSource".enumDescriptions)
    foreach ($ProblemsSourceEnumDescription in $ProblemsSourceEnumDescriptions) {
        if ([string]$ProblemsSourceEnumDescription -match "eng-cli|lsp-snapshot|snapshot path|metadata") {
            throw "VS Code problemsSource enum descriptions must use user-facing wording, not implementation details"
        }
    }
    if ($null -ne $Properties."englang.diagnosticsBackend") {
        throw "VS Code extension must not expose deprecated diagnosticsBackend in Settings; keep it as a code-only compatibility alias"
    }
    $LintOnChangeDescription = [string]$Properties."englang.lintOnChange".description
    if ($LintOnChangeDescription -match "eng-lsp|snapshot") {
        throw "VS Code lintOnChange description must avoid editor-service implementation details"
    }
    $LspPathDescription = [string]$Properties."englang.lspPath".description
    if ($LspPathDescription -match "editor service") {
        throw "VS Code lspPath description must describe live editor features instead of editor-service internals"
    }
    $SemanticDescription = [string]$Properties."englang.semanticHighlighting.enabled".description
    if ($SemanticDescription -match "eng-lsp|snapshot|semantic tokens") {
        throw "VS Code semanticHighlighting description must avoid editor-service implementation details"
    }
    foreach ($ForbiddenEditorDocWording in @(
        "LSP-client extension",
        "editor-service smoke checks",
        "short editor-service bridge",
        "editor-service snapshot",
        "smoke/snapshot tooling",
        "stable persistent editor-service contract",
        "Packaged LSP smoke/snapshot",
        "editor-service smoke and snapshot paths",
        "LSP smoke/snapshot tooling",
        "persistent LSP integration"
    )) {
        if ($VscodeReadmeSource.Contains($ForbiddenEditorDocWording) -or $NativeIdeHowtoSource.Contains($ForbiddenEditorDocWording) -or $UserGuideSource.Contains($ForbiddenEditorDocWording) -or $FeatureMaturitySource.Contains($ForbiddenEditorDocWording) -or $MainInternalStatusSource.Contains($ForbiddenEditorDocWording) -or $CurrentStatusSource.Contains($ForbiddenEditorDocWording) -or $CurrentTracksSource.Contains($ForbiddenEditorDocWording)) {
            throw "User-facing editor docs must avoid internal wording: $ForbiddenEditorDocWording"
        }
    }
    foreach ($ForbiddenPublicStatusWording in @(
        "implementation seed",
        "internal syntax seeds",
        "Smoke/snapshot tooling"
    )) {
        if ($FeatureMaturitySource.Contains($ForbiddenPublicStatusWording) -or $MainInternalStatusSource.Contains($ForbiddenPublicStatusWording)) {
            throw "Current status docs must avoid stale implementation wording: $ForbiddenPublicStatusWording"
        }
    }
    if ($NativeIdeHowtoSource.Contains("semantic-token legend") -or $NativeIdeHowtoSource.Contains("semantic token type/modifiers")) {
        throw "Native IDE user how-to must describe highlight UI in user-facing terms"
    }
    $RiskDecorationDescription = [string]$Properties."englang.reviewRiskDecorations.enabled".description
    if ($RiskDecorationDescription -notmatch "review risks") {
        throw "VS Code reviewRiskDecorations setting must describe review-risk markers"
    }
    $SemanticModifiers = @($Package.contributes.semanticTokenModifiers | ForEach-Object { $_.id })
    foreach ($RequiredSemanticModifier in @(
        "unit", "quantity", "axis", "timeseries", "uncertain",
        "sideEffect", "external", "validation", "report", "solver",
        "planned", "internal", "riskHigh", "riskMedium", "state", "input",
        "model", "db", "cache", "workflowStep"
    )) {
        if ($SemanticModifiers -notcontains $RequiredSemanticModifier) {
            throw "VS Code extension missing semantic token modifier $RequiredSemanticModifier"
        }
    }
    foreach ($SemanticModifier in $SemanticModifiers) {
        if (-not $TokenScopesDoc.Contains($SemanticModifier)) {
            throw "editor token scope contract missing semantic modifier $SemanticModifier"
        }
    }
    $SemanticScopeRule = @($Package.contributes.semanticTokenScopes | Where-Object { $_.language -eq "englang" }) | Select-Object -First 1
    if ($null -eq $SemanticScopeRule) {
        throw "VS Code extension missing englang semantic token scope mappings"
    }
    foreach ($RequiredTokenScope in @(
        "type", "type.unit", "type.quantity", "property.unit", "variable.quantity", "function.external",
        "method.sideEffect", "property.sideEffect", "variable.validation", "variable.report",
        "keyword.sideEffect", "keyword.external", "keyword.validation",
        "keyword.report", "keyword.solver", "function.solver",
        "property.external", "property.solver", "keyword.deprecated", "class.deprecated", "variable.state",
        "parameter.input", "variable.riskHigh", "keyword.riskHigh", "class.riskHigh",
        "property.riskHigh", "variable.riskMedium", "keyword.riskMedium", "class.riskMedium",
        "property.riskMedium", "variable.model", "variable.db", "property.db",
        "function.model", "keyword.model", "function.defaultLibrary", "namespace.defaultLibrary",
        "namespace.imported", "namespace.internal", "type.axis", "variable.cache", "keyword.workflowStep",
        "function.workflowStep"
    )) {
        $ScopeProperty = $SemanticScopeRule.scopes.PSObject.Properties[$RequiredTokenScope]
        if ($null -eq $ScopeProperty -or @($ScopeProperty.Value).Count -eq 0) {
            throw "VS Code extension missing semantic token scope mapping $RequiredTokenScope"
        }
    }
    $SemanticFallbackScopes = $SemanticScopeRule.scopes.PSObject.Properties |
        ForEach-Object { @($_.Value) } |
        Sort-Object -Unique
    foreach ($SemanticFallbackScope in $SemanticFallbackScopes) {
        if (-not $TokenScopesDoc.Contains($SemanticFallbackScope)) {
            throw "editor token scope contract missing semantic fallback scope $SemanticFallbackScope"
        }
    }
    $ConfigurationDefaults = $Package.contributes.configurationDefaults
    if ($null -eq $ConfigurationDefaults) {
        throw "VS Code extension missing configurationDefaults contribution"
    }
    $EngLangDefaultsProperty = $ConfigurationDefaults.PSObject.Properties["[englang]"]
    if ($null -eq $EngLangDefaultsProperty -or $EngLangDefaultsProperty.Value."editor.semanticHighlighting.enabled" -ne $true) {
        throw "VS Code extension must enable editor.semanticHighlighting.enabled by default for englang"
    }
    if ($null -ne $Properties."englang.runEntry") {
        throw "VS Code extension must not expose deprecated englang.runEntry configuration"
    }
    $ExtensionSource = Get-Content -LiteralPath $ExtensionJsPath -Raw
    $CompletionProviderSource = Get-Content -LiteralPath $CompletionProviderPath -Raw
    $LocalCodeActionsSource = Get-Content -LiteralPath $LocalCodeActionsPath -Raw
    $LspCodeActionsSource = Get-Content -LiteralPath $LspCodeActionsPath -Raw
    $LspKindsSource = Get-Content -LiteralPath $LspKindsPath -Raw
    $LspNavigationSource = Get-Content -LiteralPath $LspNavigationPath -Raw
    $LspRangesSource = Get-Content -LiteralPath $LspRangesPath -Raw
    $LspSemanticTokensSource = Get-Content -LiteralPath $LspSemanticTokensPath -Raw
    $ArtifactRegistrySource = Get-Content -LiteralPath $ArtifactRegistryPath -Raw
    $EditorMetadataLoaderSource = Get-Content -LiteralPath $EditorMetadataLoaderPath -Raw
    $ExecutionProfilesSource = Get-Content -LiteralPath $ExecutionProfilesPath -Raw
    $ModuleStatusSource = Get-Content -LiteralPath $ModuleStatusPath -Raw
    foreach ($ForbiddenCommandWording in @(
        "Current File Review JSON",
        "Last Run Review JSON",
        "Last Run Output Manifest",
        "Last Run Process Results",
        "Last Run Result Artifact",
        "Last Run Report Spec",
        "Last Static Run Plan",
        "Last Run Plan",
        "Last Run Lock",
        "Last Run Cache Manifest",
        "Last Run Plot Spec",
        "Last Run Plot Manifest",
        "Result Artifact",
        "Report Spec",
        "Static Run Plan",
        "Run Plan",
        "Run Lock",
        "Cache Manifest",
        "Plot Spec",
        "Plot Manifest"
    )) {
        if ($PackageSource.Contains($ForbiddenCommandWording) -or $ExtensionSource.Contains($ForbiddenCommandWording) -or $ArtifactRegistrySource.Contains($ForbiddenCommandWording)) {
            throw "VS Code command wording should use user-facing artifact names instead of '$ForbiddenCommandWording'"
        }
    }
    if (-not $ExtensionSource.Contains('require("./moduleStatus")')) {
        throw "VS Code workflow module panel must load wording helpers from moduleStatus.js"
    }
    foreach ($RequiredModuleWordingToken in @("moduleStatusDisplay", "moduleStatusDetailDisplay", "moduleBackingLabel", "Compiler/runtime", "No executable backing")) {
        if (-not ($ExtensionSource.Contains($RequiredModuleWordingToken) -or $ModuleStatusSource.Contains($RequiredModuleWordingToken))) {
            throw "VS Code workflow module panel missing wording token $RequiredModuleWordingToken"
        }
    }
    if ($ExtensionSource.Contains("function moduleStatusDisplay") -or $ExtensionSource.Contains("function moduleBackingLabel")) {
        throw "VS Code extension must keep workflow module wording helpers in moduleStatus.js"
    }
    if ($ExtensionSource.Contains('reviewValue(module, "backing")')) {
        throw "VS Code workflow module panel must not display raw registry backing keys"
    }
    if ($ExtensionSource.Contains("--entry") -or $ExtensionSource.Contains("runEntry")) {
        throw "VS Code extension run command must use top-level execution without entry flags"
    }
    if (-not $ExtensionSource.Contains("--save-artifacts")) {
        throw "VS Code extension run command must save artifacts for review/open-artifact commands"
    }
    if (-not $ExtensionSource.Contains("--profile") -or -not $ExtensionSource.Contains("executionProfile") -or -not $ExtensionSource.Contains("switchExecutionProfile")) {
        throw "VS Code extension run command must expose and pass an execution profile"
    }
    if (-not $ExtensionSource.Contains('require("./executionProfiles")') -or -not $ExecutionProfilesSource.Contains("EXECUTION_PROFILES") -or -not $ExecutionProfilesSource.Contains('"normal"') -or -not $ExecutionProfilesSource.Contains('"safe"') -or -not $ExecutionProfilesSource.Contains('"repro"')) {
        throw "VS Code extension must load user-facing execution profiles from executionProfiles.js"
    }
    if ($ExtensionSource.Contains("const EXECUTION_PROFILES = [")) {
        throw "VS Code extension must keep execution profile labels in executionProfiles.js"
    }
    if (-not $ExtensionSource.Contains("runExample") -or -not $ExtensionSource.Contains("findExampleFiles") -or -not $ExtensionSource.Contains('"official"') -or -not $ExtensionSource.Contains('"workflows"')) {
        throw "VS Code extension must expose an example runner for official and workflow examples"
    }
    if (-not $ExtensionSource.Contains('"review", document.uri.fsPath, "--json"')) {
        throw "VS Code extension must expose a current-file review JSON command"
    }
    if (-not $ExtensionSource.Contains("openReviewPanel") -or -not $ExtensionSource.Contains("createWebviewPanel") -or -not $ExtensionSource.Contains("renderReviewSummaryHtml")) {
        throw "VS Code extension must expose a current-file review summary panel"
    }
    if (-not $ExtensionSource.Contains("<h2>Inputs</h2>") -or -not $ExtensionSource.Contains("<h2>Schemas</h2>") -or -not $ExtensionSource.Contains("<h2>Units And Quantities</h2>") -or -not $ExtensionSource.Contains("<h2>Derived Values</h2>") -or -not $ExtensionSource.Contains("<h2>Caches</h2>")) {
        throw "VS Code extension review panel must expose core ReviewDocument sections"
    }
    if (-not $ExtensionSource.Contains("<h2>Review Fingerprint</h2>")) {
        throw "VS Code extension review panel must label semantic_hash as Review Fingerprint"
    }
    if ($ExtensionSource.Contains("<h2>Semantic Hash</h2>")) {
        throw "VS Code extension review panel must not expose internal Semantic Hash wording"
    }
    if (-not $ExtensionSource.Contains("onDidReceiveMessage") -or -not $ExtensionSource.Contains("data-source-line") -or -not $ExtensionSource.Contains("openSourceLine")) {
        throw "VS Code extension review panel must support source-line navigation"
    }
    foreach ($RequiredSourceLineToken in @(
        "function lineValue(item)",
        "source_span",
        "sourceSpan",
        "source_line",
        "sourceLine",
        "reviewRiskLineNumber"
    )) {
        if (-not $ExtensionSource.Contains($RequiredSourceLineToken)) {
            throw "VS Code extension review panel missing normalized source-line token $RequiredSourceLineToken"
        }
    }
    if (-not $ExtensionSource.Contains("reviewPanelArtifacts") -or -not $ExtensionSource.Contains("data-artifact-id") -or -not $ExtensionSource.Contains("openArtifact")) {
        throw "VS Code extension review panel must expose clickable last-run artifacts"
    }
    if (-not $ExtensionSource.Contains('require("./artifactRegistry")') -or -not $ArtifactRegistrySource.Contains("LAST_RUN_ARTIFACTS") -or -not $ArtifactRegistrySource.Contains("Report HTML") -or -not $ArtifactRegistrySource.Contains("Output List")) {
        throw "VS Code extension must load user-facing artifact labels from artifactRegistry.js"
    }
    if (-not $ExtensionSource.Contains("onDidChangeTextDocument") -or -not $ExtensionSource.Contains("--snapshot-stdin")) {
        throw "VS Code extension must support debounced unsaved-buffer diagnostics through eng-lsp --snapshot-stdin"
    }
    foreach ($RequiredSnapshotReuseToken in @(
        "const snapshotPromiseCache = new Map();",
        "const cached = snapshotPromiseCache.get(key);",
        "snapshotPromiseCache.set(key, promise);",
        "promise.finally(() =>",
        "function snapshotCacheKey(document)",
        "document.version !== documentVersion",
        "snapshotPromiseCache.delete(key)"
    )) {
        if (-not $ExtensionSource.Contains($RequiredSnapshotReuseToken)) {
            throw "VS Code extension missing shared LSP snapshot reuse token $RequiredSnapshotReuseToken"
        }
    }
    if (([regex]::Matches($ExtensionSource, [regex]::Escape("clearSnapshotCache(document)"))).Count -lt 3) {
        throw "VS Code extension must clear shared LSP snapshot cache on document changes and close"
    }
    foreach ($RequiredLiveEditorOutputToken in @(
        "Live editor check failed:",
        "Unable to parse EngLang live editor data:",
        "Completion lookup failed:",
        "Unable to parse EngLang completion data:",
        "Definition lookup failed:",
        "Unable to parse EngLang definition data:"
    )) {
        if (-not $ExtensionSource.Contains($RequiredLiveEditorOutputToken)) {
            throw "VS Code extension missing user-facing live editor output token $RequiredLiveEditorOutputToken"
        }
    }
    foreach ($ForbiddenLiveEditorOutputToken in @(
        "LSP snapshot failed:",
        "Unable to parse EngLang LSP snapshot:",
        "completion snapshot failed:",
        "Unable to parse EngLang completion snapshot:",
        "definition snapshot failed:",
        "Unable to parse EngLang definition snapshot:"
    )) {
        if ($ExtensionSource.Contains($ForbiddenLiveEditorOutputToken)) {
            throw "VS Code extension output must use live editor wording, not $ForbiddenLiveEditorOutputToken"
        }
    }
    foreach ($RequiredHoverToken in @(
        "new EngHoverProvider(context)",
        "async provideHover",
        "findHoverForWord",
        "hoverNameMatches",
        "snapshotDocumentSource(document, this.context, cancellationToken)",
        "reviewCache.set(document.uri.fsPath, snapshot)",
        "hover.status",
        "hover.kind"
    )) {
        if (-not $ExtensionSource.Contains($RequiredHoverToken)) {
            throw "VS Code extension missing live hover snapshot token $RequiredHoverToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./editorMetadata")') -or -not $ExtensionSource.Contains("loadEditorMetadata(__dirname)")) {
        throw "VS Code extension must load editor metadata through editorMetadata.js"
    }
    if (-not $EditorMetadataLoaderSource.Contains("englang-editor-metadata.json") -or -not $EditorMetadataLoaderSource.Contains("semantic_token_legend") -or -not $EditorMetadataLoaderSource.Contains("completion_seed")) {
        throw "VS Code editor metadata loader must read generated semantic legend and completion seed metadata"
    }
    if ($ExtensionSource.Contains("const SEMANTIC_TOKEN_TYPES = [") -or $ExtensionSource.Contains("const SEMANTIC_TOKEN_MODIFIERS = [")) {
        throw "VS Code extension must not hardcode semantic token legend arrays"
    }
    $CompletionSource = $ExtensionSource + "`n" + $CompletionProviderSource
    if (-not $ExtensionSource.Contains("COMPLETION_SEED") -or -not $CompletionProviderSource.Contains("completion.lsp_kind")) {
        throw "VS Code extension must use generated completion seed metadata as the completion fallback"
    }
    foreach ($RequiredCompletionToken in @(
        'require("./completionProvider")',
        "EngCompletionProvider",
        "completionSnapshotForPosition",
        "cachedSnapshotForDocument",
        "completionItemsFromPayload",
        "completionKindFromLsp",
        "new vscode.CompletionItem"
    )) {
        if (-not $CompletionSource.Contains($RequiredCompletionToken)) {
            throw "VS Code extension missing completion provider token $RequiredCompletionToken"
        }
    }
    if (-not $ExtensionSource.Contains("showSemanticTokensDebug") -or -not $ExtensionSource.Contains("token_counts_by_type") -or -not $ExtensionSource.Contains("token_counts_by_modifier") -or -not $ExtensionSource.Contains("token_samples_by_type") -or -not $ExtensionSource.Contains("token_samples_by_modifier")) {
        throw "VS Code extension must expose semantic token debug output"
    }
    if (-not $ExtensionSource.Contains('require("./lspSemanticTokens")') -or -not $LspSemanticTokensSource.Contains("semanticTokensFromSnapshot") -or -not $LspSemanticTokensSource.Contains("semanticTokenRange") -or -not $LspSemanticTokensSource.Contains("semanticTokenDebugSample")) {
        throw "VS Code extension must share LSP semantic token conversion through lspSemanticTokens.js"
    }
    if ($ExtensionSource.Contains("function semanticTokensFromSnapshot") -or $ExtensionSource.Contains("function semanticModifierBits") -or $ExtensionSource.Contains("function semanticTokenDebugSample")) {
        throw "VS Code extension must keep LSP semantic token conversion in lspSemanticTokens.js"
    }
    foreach ($RequiredHighlightDebugToken in @("highlight_count", "highlight_counts_by_category", "highlight_counts_by_detail", "highlight_samples_by_category", "highlight_samples_by_detail", "highlight_data")) {
        if (-not $ExtensionSource.Contains($RequiredHighlightDebugToken)) {
            throw "VS Code highlight inspection output missing user-facing token $RequiredHighlightDebugToken"
        }
    }
    if ($ExtensionSource.Contains("No semantic token snapshot is available")) {
        throw "VS Code highlight inspection warning must use highlight wording"
    }
    foreach ($RequiredRiskDecorationToken in @(
        "createReviewRiskDecorationTypes",
        "updateReviewRiskDecorations",
        "reviewRiskDecorationOptions",
        "setReviewRiskDecorationLine",
        "englang.reviewRiskDecorations.enabled",
        "riskHigh",
        "riskMedium",
        "editorError.foreground",
        "editorWarning.foreground",
        "OverviewRulerLane.Right"
    )) {
        if (-not $ExtensionSource.Contains($RequiredRiskDecorationToken)) {
            throw "VS Code extension missing review risk decoration token $RequiredRiskDecorationToken"
        }
    }
    $SemanticSymbolDecorationSource = $ExtensionSource + "`n" + $LspSemanticTokensSource
    foreach ($RequiredSemanticSymbolDecorationToken in @(
        "createSemanticSymbolDecorationTypes",
        "updateSemanticSymbolDecorations",
        "semanticSymbolDecorationOptions",
        "semanticTokenRange",
        "semanticSymbolHoverMessage",
        "semanticSymbolDecorations",
        "underline dotted",
        "internal",
        "planned"
    )) {
        if (-not $SemanticSymbolDecorationSource.Contains($RequiredSemanticSymbolDecorationToken)) {
            throw "VS Code extension missing semantic symbol decoration token $RequiredSemanticSymbolDecorationToken"
        }
    }
    $NavigationSource = $ExtensionSource + "`n" + $LspNavigationSource
    foreach ($RequiredDefinitionToken in @(
        "registerDefinitionProvider",
        "EngDefinitionProvider",
        "definitionSnapshotForPosition",
        "--definition-stdin",
        "definitionLocationFromLsp",
        "vscode.Uri.parse",
        "definitionNameCandidates"
    )) {
        if (-not $NavigationSource.Contains($RequiredDefinitionToken)) {
            throw "VS Code extension missing live definition token $RequiredDefinitionToken"
        }
    }
    foreach ($RequiredWorkspaceSymbolToken in @(
        "registerWorkspaceSymbolProvider",
        "EngWorkspaceSymbolProvider",
        "workspaceSymbolsForQuery",
        "workspaceSymbolsForFolder",
        "--workspace-symbols",
        "workspaceSymbolInformationFromLsp",
        "findLspRuntimeForRoot",
        "new vscode.SymbolInformation"
    )) {
        if (-not $NavigationSource.Contains($RequiredWorkspaceSymbolToken)) {
            throw "VS Code extension missing workspace symbol token $RequiredWorkspaceSymbolToken"
        }
    }
    foreach ($RequiredFormattingToken in @(
        "registerDocumentFormattingEditProvider",
        "EngFormattingProvider",
        "formatDocumentSource",
        "--format-stdin",
        "fullDocumentRange",
        "vscode.TextEdit.replace"
    )) {
        if (-not $ExtensionSource.Contains($RequiredFormattingToken)) {
            throw "VS Code extension missing formatting token $RequiredFormattingToken"
        }
    }
    $QuickFixSource = $ExtensionSource + "`n" + $LocalCodeActionsSource + "`n" + $LspCodeActionsSource
    foreach ($RequiredQuickFixToken in @(
        "registerCodeActionsProvider",
        "--code-actions-stdin",
        "codeActionsForDocumentSource",
        "lspCodeActionsFromPayload",
        "workspaceEditFromLspCodeAction",
        "lspRange.start.character",
        "diagnostic.range.start.character",
        "lspRange.end.character",
        "diagnostic.range.end.character",
        "localCodeActions",
        "E-SYNTAX-DECL-001",
        "E-STRUCT-ARGS-001",
        "E-EQ-BOOL-001",
        "E-SCRIPT-001",
        "E-CMD-AMBIG-001",
        "W-QTY-AMBIG-001",
        "E-DIM-ADD-",
        "E-PUBLIC-ANNOTATION-001",
        "E-FS-CONFIRM-001",
        "E-FS-DELETE-001",
        "E-NET-HASH-MISMATCH",
        "E-WITH-OPTION-001",
        "E-NET-RETRY-POLICY",
        "E-NET-TIMEOUT",
        "E-NET-BODY-SIZE-LIMIT",
        "E-PROCESS-RETRY-POLICY",
        "E-PROCESS-TIMEOUT",
        "E-PROCESS-ALLOW-FAILURE",
        "removeScriptWrapperAction",
        "quantityAnnotationActions",
        "missingUnitActions",
        "schemaAnnotationAction",
        "fileMutationConfirmAction",
        "recursiveDeleteAction",
        "expectedSha256Action",
        "expectedSha256FromDiagnostic",
        "commandTargetParenthesesAction",
        "commandTargetFromDiagnostic",
        "plotUnitOptionAction",
        "confidenceBandOptionAction",
        "withOptionRenameAction",
        "unknownWithOptionName",
        "Use plot y-axis option: unit y =",
        "Use confidence band option: confidence_band =",
        "optionQuickFix",
        "optionValueReplacementAction",
        "optionAssignmentRange"
    )) {
        if (-not $QuickFixSource.Contains($RequiredQuickFixToken)) {
            throw "VS Code extension missing quick fix token $RequiredQuickFixToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./localCodeActions")') -or -not $LocalCodeActionsSource.Contains("localCodeActions") -or -not $LocalCodeActionsSource.Contains("diagnosticCode")) {
        throw "VS Code extension must load local quick fix helpers from localCodeActions.js"
    }
    if (-not $ExtensionSource.Contains('require("./lspCodeActions")') -or -not $LspCodeActionsSource.Contains("lspCodeActionsFromPayload") -or -not $LspCodeActionsSource.Contains("workspaceEditFromLspCodeAction")) {
        throw "VS Code extension must load LSP quick fix bridge helpers from lspCodeActions.js"
    }
    if (-not $ExtensionSource.Contains('require("./lspKinds")') -or -not $LspKindsSource.Contains("symbolKindFromLsp") -or -not $LspKindsSource.Contains("completionKindFromLsp") -or -not $LspKindsSource.Contains("foldingRangeKindFromLsp")) {
        throw "VS Code extension must share LSP kind conversion through lspKinds.js"
    }
    if (-not $CompletionProviderSource.Contains('require("./lspKinds")')) {
        throw "VS Code completion provider must reuse shared LSP kind conversion"
    }
    if (-not $ExtensionSource.Contains('require("./lspNavigation")') -or -not $LspNavigationSource.Contains("definitionLocationFromLsp") -or -not $LspNavigationSource.Contains("definitionLocationFromSnapshotSymbols") -or -not $LspNavigationSource.Contains("documentSymbolsFromSnapshot") -or -not $LspNavigationSource.Contains("workspaceSymbolInformationFromLsp") -or -not $LspNavigationSource.Contains("definitionNameCandidates")) {
        throw "VS Code extension must share LSP navigation conversion through lspNavigation.js"
    }
    if (-not $LspNavigationSource.Contains('require("./lspKinds")') -or -not $LspNavigationSource.Contains('require("./lspRanges")')) {
        throw "VS Code LSP navigation bridge must reuse shared kind and range conversion"
    }
    if (-not $LspCodeActionsSource.Contains('require("./lspRanges")') -or -not $LspNavigationSource.Contains('require("./lspRanges")') -or -not $LspRangesSource.Contains("vscodeRangeFromLsp")) {
        throw "VS Code extension must share LSP range conversion through lspRanges.js"
    }
    if ($ExtensionSource.Contains("function localCodeActions") -or $ExtensionSource.Contains("function optionQuickFix") -or $ExtensionSource.Contains("function quantityAnnotationActions") -or $ExtensionSource.Contains("function removeScriptWrapperAction")) {
        throw "VS Code extension must keep local quick fix helpers in localCodeActions.js"
    }
    if ($ExtensionSource.Contains("function lspCodeActionsFromPayload") -or $ExtensionSource.Contains("function workspaceEditFromLspCodeAction") -or $ExtensionSource.Contains("function lspDiagnosticMatchesVscode")) {
        throw "VS Code extension must keep LSP quick fix bridge helpers in lspCodeActions.js"
    }
    if ($ExtensionSource.Contains("function symbolKindFromLsp") -or $ExtensionSource.Contains("function completionKindFromLsp") -or $ExtensionSource.Contains("function foldingRangeKindFromLsp")) {
        throw "VS Code extension must keep LSP kind conversion in lspKinds.js"
    }
    if ($ExtensionSource.Contains("class EngCompletionProvider") -or $ExtensionSource.Contains("function addCompletion") -or $ExtensionSource.Contains("function completionItemsFromPayload")) {
        throw "VS Code extension must keep completion provider helpers in completionProvider.js"
    }
    if ($ExtensionSource.Contains("function definitionLocationFromLsp") -or $ExtensionSource.Contains("function definitionLocationFromSnapshotSymbols") -or $ExtensionSource.Contains("function workspaceSymbolInformationFromLsp") -or $ExtensionSource.Contains("function documentSymbolsFromSnapshot") -or $ExtensionSource.Contains("function definitionNameCandidates") -or $ExtensionSource.Contains("function identifierPathRangeAt")) {
        throw "VS Code extension must keep LSP navigation conversion in lspNavigation.js"
    }
    if ($ExtensionSource.Contains("function vscodeRangeFromLsp") -or $LspCodeActionsSource.Contains("function vscodeRangeFromLsp")) {
        throw "VS Code extension must keep LSP range conversion in lspRanges.js"
    }
    foreach ($RequiredProblemsSourceToken in @("function problemsSource(document)", 'explicitlyConfiguredEngValue(config, "problemsSource")', 'return source === "live" ? "lsp-snapshot" : "eng-cli"', "diagnosticsBackendLabel(backend)")) {
        if (-not $ExtensionSource.Contains($RequiredProblemsSourceToken)) {
            throw "VS Code extension missing user-facing Problems source token $RequiredProblemsSourceToken"
        }
    }
    $ProblemsSourceEnum = @($Properties."englang.problemsSource".enum)
    foreach ($RequiredProblemsSource in @("file", "live")) {
        if ($ProblemsSourceEnum -notcontains $RequiredProblemsSource) {
            throw "VS Code extension problemsSource missing enum value $RequiredProblemsSource"
        }
    }
    if (@($Properties."englang.problemsSource".enumDescriptions).Count -lt 2) {
        throw "VS Code extension problemsSource must include user-facing enum descriptions"
    }
    foreach ($RequiredLegacyProblemsToken in @('config.get("diagnosticsBackend", "eng-cli")', 'legacyBackend === "lsp-snapshot" ? "live" : "file"')) {
        if (-not $ExtensionSource.Contains($RequiredLegacyProblemsToken)) {
            throw "VS Code extension must keep diagnosticsBackend as a code-only compatibility alias"
        }
    }
    $ProfileEnum = @($Properties."englang.executionProfile".enum)
    foreach ($RequiredProfile in @("normal", "safe", "repro")) {
        if ($ProfileEnum -notcontains $RequiredProfile) {
            throw "VS Code extension executionProfile missing enum value $RequiredProfile"
        }
    }
    if (@($Properties."englang.executionProfile".enumDescriptions).Count -lt 3) {
        throw "VS Code extension executionProfile must include user-facing enum descriptions"
    }

    $LspSource = Get-Content -LiteralPath $LspSourcePath -Raw
    $LspCliSource = Get-Content -LiteralPath $LspCliSourcePath -Raw
    foreach ($RequiredLspDefinitionToken in @(
        "--definition-stdin",
        "command_definition_stdin",
        "definition_for_request",
        "imported_definition_target"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspDefinitionToken)) {
            throw "eng-lsp CLI missing stdin definition token $RequiredLspDefinitionToken"
        }
    }
    foreach ($RequiredLspSemanticToken in @(
        '"range": true',
        "textDocument/semanticTokens/full",
        "textDocument/semanticTokens/range",
        "semantic_tokens_range_for_request",
        "semantic_token_intersects_range"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspSemanticToken)) {
            throw "eng-lsp CLI missing semantic token protocol token $RequiredLspSemanticToken"
        }
    }
    foreach ($RequiredLspWorkspaceSymbolToken in @(
        "workspaceSymbolProvider",
        "workspace/symbol",
        "--workspace-symbols",
        "command_workspace_symbols",
        "workspace_symbols_for_request",
        "MAX_WORKSPACE_SYMBOL_FILES",
        "MAX_WORKSPACE_SYMBOL_RESULTS"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspWorkspaceSymbolToken)) {
            throw "eng-lsp CLI missing workspace symbol token $RequiredLspWorkspaceSymbolToken"
        }
    }
    foreach ($RequiredLspCodeActionToken in @(
        "codeActionProvider",
        "textDocument/codeAction",
        "code_actions_for_request",
        "lsp_replacement_code_action",
        "lsp_remove_script_wrapper_code_action",
        "lsp_quantity_annotation_code_actions",
        "lsp_missing_unit_code_actions",
        "lsp_schema_annotation_code_action",
        "lsp_file_mutation_confirm_code_action",
        "lsp_recursive_delete_code_action",
        "lsp_option_value_replacement_code_action",
        "lsp_with_option_rename_code_action",
        "lsp_parenthesize_command_target_code_action",
        "matching_block_end_line",
        "E-SYNTAX-DECL-001",
        "E-STRUCT-ARGS-001",
        "E-SCRIPT-001",
        "E-EQ-BOOL-001",
        "E-CMD-AMBIG-001",
        "W-QTY-AMBIG-001",
        "E-DIM-ADD-",
        "E-PUBLIC-ANNOTATION-001",
        "E-FS-CONFIRM-001",
        "E-FS-DELETE-001",
        "E-NET-RETRY-POLICY",
        "E-PROCESS-ALLOW-FAILURE"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspCodeActionToken)) {
            throw "eng-lsp CLI missing code action protocol token $RequiredLspCodeActionToken"
        }
    }
    foreach ($RequiredLspFormattingToken in @(
        "--format-stdin",
        "documentFormattingProvider",
        "textDocument/formatting",
        "formatting_edits_for_request",
        "full_document_range",
        "format_source"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspFormattingToken)) {
            throw "eng-lsp CLI missing formatting protocol token $RequiredLspFormattingToken"
        }
    }
    if (-not $LspCliSource.Contains("--code-actions-stdin") -or -not $LspCliSource.Contains("command_code_actions_stdin")) {
        throw "eng-lsp CLI missing stdin code action bridge"
    }
    $EditorMetadata = Get-Content -LiteralPath $EditorMetadataPath -Raw | ConvertFrom-Json
    $GeneratedCompletions = Get-Content -LiteralPath $CompletionsPath -Raw | ConvertFrom-Json
    if ($EditorMetadata.format -ne "eng-lsp-editor-metadata-v1") {
        throw "generated VS Code editor metadata returned unexpected format $($EditorMetadata.format)"
    }
    if ($GeneratedCompletions.format -ne "eng-lsp-editor-metadata-v1") {
        throw "generated VS Code completions returned unexpected format $($GeneratedCompletions.format)"
    }
    $MetadataCompletionLabels = @($EditorMetadata.completion_seed | ForEach-Object { $_.label })
    $GeneratedCompletionLabels = @($GeneratedCompletions.completion_seed | ForEach-Object { $_.label })
    Assert-SameStringSequence -Left $MetadataCompletionLabels -Right $GeneratedCompletionLabels -Description "VS Code generated completion seed labels"
    if ($MetadataCompletionLabels.Count -lt 100) {
        throw "generated VS Code completion seed is unexpectedly small: $($MetadataCompletionLabels.Count)"
    }
    foreach ($RequiredCompletion in @("records", "promote json records", "read json", "eng.table", "split")) {
        $Completion = @($EditorMetadata.completion_seed | Where-Object { $_.label -eq $RequiredCompletion }) | Select-Object -First 1
        if ($null -eq $Completion) {
            throw "generated VS Code editor metadata missing completion seed $RequiredCompletion"
        }
        if ($null -eq $Completion.lsp_kind) {
            throw "generated VS Code editor metadata completion seed $RequiredCompletion missing lsp_kind"
        }
    }
    $GeneratedSemanticTypes = @($EditorMetadata.semantic_token_legend.token_types)
    $GeneratedSemanticModifiers = @($EditorMetadata.semantic_token_legend.token_modifiers)
    $LspSemanticTypes = Read-RustStringSliceConst -Source $LspSource -Name "SEMANTIC_TOKEN_TYPES"
    $LspSemanticModifiers = Read-RustStringSliceConst -Source $LspSource -Name "SEMANTIC_TOKEN_MODIFIERS"
    Assert-SameStringSequence -Left $GeneratedSemanticTypes -Right $LspSemanticTypes -Description "VS Code generated/LSP semantic token types"
    Assert-SameStringSequence -Left $GeneratedSemanticModifiers -Right $LspSemanticModifiers -Description "VS Code generated/LSP semantic token modifiers"
    $StandardSemanticModifiers = @("declaration", "definition", "readonly", "static", "local", "imported", "defaultLibrary", "deprecated")
    foreach ($Modifier in $LspSemanticModifiers) {
        if ($StandardSemanticModifiers -notcontains $Modifier -and $SemanticModifiers -notcontains $Modifier) {
            throw "VS Code package.json missing custom semantic token modifier from LSP legend: $Modifier"
        }
    }

    $Node = Get-Command node -ErrorAction SilentlyContinue
    if ($null -ne $Node) {
        try {
            Invoke-Native $Node.Source "--check" $ExtensionJsPath
            Invoke-Native $Node.Source "--check" $CompletionProviderPath
            Invoke-Native $Node.Source "--check" $LocalCodeActionsPath
            Invoke-Native $Node.Source "--check" $LspCodeActionsPath
            Invoke-Native $Node.Source "--check" $LspKindsPath
            Invoke-Native $Node.Source "--check" $LspNavigationPath
            Invoke-Native $Node.Source "--check" $LspRangesPath
            Invoke-Native $Node.Source "--check" $LspSemanticTokensPath
            Invoke-Native $Node.Source "--check" $ArtifactRegistryPath
            Invoke-Native $Node.Source "--check" $EditorMetadataLoaderPath
            Invoke-Native $Node.Source "--check" $ExecutionProfilesPath
            Invoke-Native $Node.Source "--check" $ModuleStatusPath
        } catch {
            Write-Host "Node found but not executable; skipped VS Code JavaScript syntax check. $($_.Exception.Message)"
        }
    } else {
        Write-Host "Node not found; skipped VS Code JavaScript syntax check."
    }

    Write-Host "VS Code extension contract check passed."
}

function Invoke-IdeCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }

    $TauriConfigPath = Join-Path $RepoRoot "crates\eng_ide\tauri.conf.json"
    $TauriMainPath = Join-Path $RepoRoot "crates\eng_ide\src\main.rs"
    $TauriUiIndexPath = Join-Path $RepoRoot "crates\eng_ide\ui\index.html"
    $TauriUiAppPath = Join-Path $RepoRoot "crates\eng_ide\ui\app.js"
    $TauriUiStylesPath = Join-Path $RepoRoot "crates\eng_ide\ui\styles.css"
    $LspSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\lib.rs"
    if (-not (Test-Path $TauriConfigPath)) {
        throw "missing Tauri IDE config at $TauriConfigPath"
    }
    if (-not (Test-Path $TauriMainPath)) {
        throw "missing Tauri IDE backend at $TauriMainPath"
    }
    if (-not (Test-Path $TauriUiIndexPath)) {
        throw "missing Tauri IDE static frontend at $TauriUiIndexPath"
    }
    if (-not (Test-Path $TauriUiAppPath)) {
        throw "missing Tauri IDE frontend script at $TauriUiAppPath"
    }
    if (-not (Test-Path $TauriUiStylesPath)) {
        throw "missing Tauri IDE frontend styles at $TauriUiStylesPath"
    }
    if (-not (Test-Path $LspSourcePath)) {
        throw "missing LSP source at $LspSourcePath"
    }
    $IdeUiSource = Get-Content -LiteralPath $TauriUiAppPath -Raw
    foreach ($RequiredIdeToken in @(
        "runHistory",
        "recordRunHistory",
        "renderRunHistory",
        "openPathButton",
        "renderOutputPathList",
        "selectedWorkflowNodeId",
        "renderWorkflowNodeDetail",
        "panelArtifactEmptyState",
        "No network or cache records yet.",
        "sourceBreadcrumbs",
        "source-breadcrumbs",
        "rawJsonToggle",
        "compactObjectSummary",
        "problemQuery",
        "problemQueryInput",
        "filteredProblems",
        "moduleCategory",
        "moduleQueryInput",
        "filteredModules",
        "data-module-category",
        "data-problem-line",
        "RUN_HISTORY_STORAGE_PREFIX",
        "data-open-file-path",
        "data-open-path",
        "ide_open_path",
        "editorHighlight",
        "renderHighlightedSource",
        "renderHighlightPanel",
        "highlightTokenQuery",
        "highlightTokenQueryInput",
        "clearHighlightTokenFilter",
        "filteredSemanticTokens",
        "semanticTokenSearchText",
        "semanticTokenText",
        "<th>Text</th>",
        "semanticTokenPayload",
        "semanticTokens",
        "byteOffsetToCodeUnit",
        "cursorInsight",
        "renderCursorInsight",
        "bindCursorInsightActions",
        "renderCursorInsightActions",
        "data-show-highlight-panel",
        "semanticTokenAtCaret",
        "hoverForSemanticToken",
        "sourceTokenButton",
        "selectSourceTokenRange",
        "data-source-token-line",
        "sourceLineRange",
        "sourceLineValue",
        "source_line",
        "sourceLine",
        "variableSourceCell",
        "variable-source-line",
        "codeUnitToByteOffset",
        "Timestamp",
        "Output Root",
        "Write Records",
        "Training Plans",
        "Prediction Runs",
        "Case Runs",
        "External Process Results"
    )) {
        if (-not $IdeUiSource.Contains($RequiredIdeToken)) {
            throw "Native IDE UI missing contract token $RequiredIdeToken"
        }
    }
    if ($IdeUiSource.Contains('<div class="panel-title compact">Process Results</div>')) {
        throw "Native IDE Effects panel must label process data as External Process Results"
    }
    foreach ($ForbiddenIdeWording in @(
        "<th>Artifact Root</th>",
        "No DB write artifact data yet.",
        'Manifests ${manifests.length}',
        'Specs ${specs.length}',
        "Prediction Manifests",
        "No model artifact data yet.",
        "No model specs.",
        "No model artifacts.",
        "No prediction manifests.",
        "No case artifact data yet.",
        "No case manifests.",
        "No DB write manifests.",
        "No quality artifact data yet.",
        "No kernel plan artifact data yet.",
        "No workflow plan artifact data yet.",
        "No side-effect artifact data yet.",
        "No network/cache artifact data yet.",
        "Raw quality JSON",
        "Raw kernel plan JSON",
        "Raw review document JSON",
        "Raw node JSON",
        "Raw effects JSON",
        "Raw network/cache JSON",
        "Raw DB JSON",
        "Raw model JSON",
        "Raw case JSON",
        "Raw semantic token JSON",
        "Semantic Hash",
        "No semantic legend entries.",
        "No semantic tokens match the current filter.",
        "No semantic tokens for the current check.",
        "Filter by token text, type, modifier",
        'Showing first 120 of ${escapeHtml(String(tokens.length))} tokens.',
        "<th>Type</th><th>Count</th>",
        "<th>Range</th><th>Text</th><th>Type</th><th>Modifiers</th>"
    )) {
        if ($IdeUiSource.Contains($ForbiddenIdeWording)) {
            throw "Native IDE UI should use task-oriented wording instead of '$ForbiddenIdeWording'"
        }
    }
    foreach ($RequiredAdvancedDataLabel in @("Advanced quality data", "Advanced kernel plan data", "Advanced review data", "Advanced node data", "Advanced effects data", "Advanced network/cache data", "Advanced DB data", "Advanced model data", "Advanced case data")) {
        if (-not $IdeUiSource.Contains($RequiredAdvancedDataLabel)) {
            throw "Native IDE UI missing task-oriented advanced data label $RequiredAdvancedDataLabel"
        }
    }
    if (-not $IdeUiSource.Contains("Review Fingerprint")) {
        throw "Native IDE review panel must label semantic_hash as Review Fingerprint"
    }
    foreach ($RequiredModuleWordingToken in @("moduleStatusDisplay", "moduleBackingLabel", "Compiler/runtime", "No executable backing")) {
        if (-not $IdeUiSource.Contains($RequiredModuleWordingToken)) {
            throw "Native IDE module wording missing token $RequiredModuleWordingToken"
        }
    }
    if ($IdeUiSource.Contains('${escapeHtml(module.status || "-")} / ${escapeHtml(module.backing || "-")}')) {
        throw "Native IDE Modules view must not display raw registry status/backing keys"
    }
    $IdeUiStyles = Get-Content -LiteralPath $TauriUiStylesPath -Raw
    foreach ($RequiredIdeStyle in @("run-history-table", "status-pill", "status-pill.completed", "status-pill.blocked", "problem-query", "problem-row", "module-toolbar", "module-query", "editor-highlight", "hl-keyword", "hl-mod-unit", "hl-mod-solver", "hl-mod-riskHigh", "semantic-token-table", "token-chip", "token-range-button", "cursor-insight", "variable-source-line")) {
        if (-not $IdeUiStyles.Contains($RequiredIdeStyle)) {
            throw "Native IDE UI missing contract style $RequiredIdeStyle"
        }
    }
    $LspSource = Get-Content -LiteralPath $LspSourcePath -Raw
    $LspSemanticTypes = Read-RustStringSliceConst -Source $LspSource -Name "SEMANTIC_TOKEN_TYPES"
    foreach ($TokenType in $LspSemanticTypes) {
        $TypeStyle = ".hl-$TokenType"
        if (-not $IdeUiStyles.Contains($TypeStyle)) {
            throw "Native IDE CSS missing semantic token type style $TypeStyle"
        }
    }
    $LspSemanticModifiers = Read-RustStringSliceConst -Source $LspSource -Name "SEMANTIC_TOKEN_MODIFIERS"
    foreach ($Modifier in $LspSemanticModifiers) {
        $ModifierStyle = ".hl-mod-$Modifier"
        if (-not $IdeUiStyles.Contains($ModifierStyle)) {
            throw "Native IDE CSS missing semantic modifier style $ModifierStyle"
        }
    }
    $IdeMainSource = Get-Content -LiteralPath $TauriMainPath -Raw
    foreach ($RequiredIdeBackendToken in @("eng_lsp", "semantic_tokens", "hovers", "editor_payload_view", "snapshot_from_report_with_source", "hover_json", "editor_completion_seed", "CompletionView::from_lsp", "native_insert_for_lsp_completion", "native_ide_completion_seed_uses_lsp_editor_seed", "check_view_surfaces_lsp_semantic_tokens")) {
        if (-not $IdeMainSource.Contains($RequiredIdeBackendToken)) {
            throw "Native IDE backend missing contract token $RequiredIdeBackendToken"
        }
    }
    foreach ($ForbiddenNativeIdeCompletionToken in @("BASE_COMPLETION_KEYWORDS", "PUBLIC_TYPE_COMPLETIONS", "WORKFLOW_BUILTIN_COMPLETIONS", "WORKFLOW_OPTION_COMPLETIONS")) {
        if ($IdeMainSource.Contains($ForbiddenNativeIdeCompletionToken)) {
            throw "Native IDE backend must use eng_lsp editor completion seed instead of $ForbiddenNativeIdeCompletionToken"
        }
    }
    foreach ($ForbiddenNativeIdeFixtureToken in @("python run.py", '"target": "python"')) {
        if ($IdeMainSource.Contains($ForbiddenNativeIdeFixtureToken)) {
            throw "Native IDE backend fixture must not expose legacy Python workflow marker $ForbiddenNativeIdeFixtureToken"
        }
    }
    $Node = Get-Command node -ErrorAction SilentlyContinue
    if ($null -ne $Node) {
        try {
            Invoke-Native $Node.Source "--check" $TauriUiAppPath
        } catch {
            Write-Host "Node found but not executable; skipped IDE app.js syntax check. $($_.Exception.Message)"
        }
    } else {
        Write-Host "Node not found; skipped IDE app.js syntax check."
    }
    Invoke-Native $cargo "check" "-p" "eng_ide"
    Invoke-Native $cargo "run" "-p" "eng_ide" "--" "--smoke"
    Assert-VscodeExtensionContract
    Write-Host "IDE check passed."
}

function Invoke-LspCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "test" "-p" "eng_lsp" "--" "--nocapture"
    Invoke-Native $cargo "run" "-p" "eng_lsp" "--" "--smoke"
    Invoke-Native $cargo "run" "-p" "eng_lsp" "--" "--snapshot-check" "examples\official\01_csv_plot\main.eng"
    $EditorMetadataOutput = & $cargo "run" "-p" "eng_lsp" "--quiet" "--" "--editor-metadata"
    if ($LASTEXITCODE -ne 0) {
        throw "eng-lsp --editor-metadata failed with exit code $LASTEXITCODE"
    }
    $EditorMetadata = ($EditorMetadataOutput | Out-String).Trim() | ConvertFrom-Json
    if ($EditorMetadata.format -ne "eng-lsp-editor-metadata-v1") {
        throw "eng-lsp --editor-metadata returned unexpected format $($EditorMetadata.format)"
    }
    foreach ($RequiredCompletion in @("records", "promote json records", "read json", "eng.table", "split")) {
        $Completion = @($EditorMetadata.completion_seed | Where-Object { $_.label -eq $RequiredCompletion }) | Select-Object -First 1
        if ($null -eq $Completion) {
            throw "eng-lsp --editor-metadata missing completion seed $RequiredCompletion"
        }
    }
    foreach ($RequiredModifier in @("workflowStep", "unit", "quantity", "solver")) {
        if (@($EditorMetadata.semantic_token_legend.token_modifiers) -notcontains $RequiredModifier) {
            throw "eng-lsp --editor-metadata missing semantic token modifier $RequiredModifier"
        }
    }
    Write-Host "LSP check passed."
}

function Invoke-JitBenchTargetCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Cargo,

        [Parameter(Mandatory = $true)]
        [string] $Source,

        [Parameter(Mandatory = $true)]
        [string] $TargetName,

        [Parameter(Mandatory = $true)]
        [string] $ExpectedStatus,

        [Parameter(Mandatory = $true)]
        [string] $CandidateFragment
    )

    $output = & $Cargo "run" "-p" "eng_cli" "--quiet" "--" "jit-bench" $Source "--iterations" "1"
    if ($LASTEXITCODE -ne 0) {
        throw "jit-bench failed for $Source with exit code $LASTEXITCODE"
    }
    $jsonText = ($output | Out-String).Trim()
    Write-Host $jsonText
    $bench = $jsonText | ConvertFrom-Json
    $target = @($bench.benchmark_targets | Where-Object {
        $_.name -eq $TargetName -and $_.status -eq $ExpectedStatus
    }) | Select-Object -First 1
    if ($null -eq $target) {
        throw "jit-bench $Source did not report $TargetName as $ExpectedStatus"
    }
    $candidate = @($target.candidates | Where-Object {
        $_ -like "*$CandidateFragment*"
    }) | Select-Object -First 1
    if ($null -eq $candidate) {
        throw "jit-bench $Source target $TargetName did not include candidate containing $CandidateFragment"
    }
}

function Invoke-JitBenchmarkCaseCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Cargo,

        [Parameter(Mandatory = $true)]
        [string] $CaseDir
    )

    $caseRoot = Join-Path $RepoRoot $CaseDir
    $expectedPath = Join-Path $caseRoot "expected.json"
    if (-not (Test-Path -LiteralPath $expectedPath)) {
        throw "benchmark case $CaseDir is missing expected.json"
    }

    $expected = Get-Content -LiteralPath $expectedPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($expected.format -ne "eng-benchmark-case-v1") {
        throw "benchmark case $CaseDir has unsupported expected.json format $($expected.format)"
    }
    $source = Join-Path $CaseDir $expected.source
    $sourcePath = Join-Path $RepoRoot $source
    if (-not (Test-Path -LiteralPath $sourcePath)) {
        throw "benchmark case $CaseDir is missing source $source"
    }
    foreach ($input in @($expected.input_data)) {
        $inputPath = Join-Path $caseRoot $input
        if (-not (Test-Path -LiteralPath $inputPath)) {
            throw "benchmark case $CaseDir is missing input data $input"
        }
    }

    $output = & $Cargo "run" "-p" "eng_cli" "--quiet" "--" "jit-bench" $source "--iterations" "1"
    if ($LASTEXITCODE -ne 0) {
        throw "jit-bench failed for benchmark case $CaseDir with exit code $LASTEXITCODE"
    }
    $jsonText = ($output | Out-String).Trim()
    Write-Host $jsonText
    $bench = $jsonText | ConvertFrom-Json
    if ($bench.format -ne "eng-jit-bench-v1") {
        throw "benchmark case $CaseDir did not emit eng-jit-bench-v1"
    }
    if ($bench.comparison_policy -ne "no-speedup-claim") {
        throw "benchmark case $CaseDir changed comparison_policy to $($bench.comparison_policy)"
    }
    if ($bench.jit.status -ne "not_available") {
        throw "benchmark case $CaseDir must not report native JIT timing"
    }
    if ($bench.interpreter.status -ne "measured") {
        throw "benchmark case $CaseDir did not report measured interpreter timing"
    }
    $runs = @($bench.interpreter.runs)
    if ($runs.Count -ne 1 -or $null -eq $runs[0].elapsed_ms -or $runs[0].elapsed_ms -lt 0) {
        throw "benchmark case $CaseDir did not record one non-negative runtime timing"
    }

    $resultPath = Join-Path $RepoRoot $runs[0].result_path
    if (-not (Test-Path -LiteralPath $resultPath)) {
        throw "benchmark case $CaseDir did not generate result artifact $resultPath"
    }
    $artifactRoot = Split-Path -Parent $resultPath
    foreach ($artifactName in @("report.html", "report_spec.json", "review.json")) {
        $artifactPath = Join-Path $artifactRoot $artifactName
        if (-not (Test-Path -LiteralPath $artifactPath)) {
            throw "benchmark case $CaseDir did not generate artifact $artifactName"
        }
    }

    foreach ($targetSpec in @($expected.expected.benchmark_targets)) {
        $target = @($bench.benchmark_targets | Where-Object {
            $_.name -eq $targetSpec.name -and $_.status -eq $targetSpec.status
        }) | Select-Object -First 1
        if ($null -eq $target) {
            throw "benchmark case $CaseDir did not report $($targetSpec.name) as $($targetSpec.status)"
        }
        if ($null -ne $targetSpec.candidate_contains -and $targetSpec.candidate_contains -ne "") {
            $candidate = @($target.candidates | Where-Object {
                $_ -like "*$($targetSpec.candidate_contains)*"
            }) | Select-Object -First 1
            if ($null -eq $candidate) {
                throw "benchmark case $CaseDir target $($targetSpec.name) did not include candidate containing $($targetSpec.candidate_contains)"
            }
        }
    }

    foreach ($sampleName in @($expected.expected.executor_samples)) {
        $sample = @($bench.kernel_executor_samples | Where-Object {
            $_.candidate -eq $sampleName -and $_.status -eq "executed" -and $_.backend -eq "interpreter-fallback"
        }) | Select-Object -First 1
        if ($null -eq $sample) {
            throw "benchmark case $CaseDir did not execute kernel sample $sampleName"
        }
    }

    $resultText = Get-Content -LiteralPath $resultPath -Raw -Encoding UTF8
    foreach ($fragment in @($expected.expected.result_contains)) {
        if (-not $resultText.Contains($fragment)) {
            throw "benchmark case $CaseDir result artifact did not contain $fragment"
        }
    }

    Write-Host "ok: benchmark case $CaseDir matched expected timing, artifact, target, and correctness checks"
}

function Invoke-JitBenchmarkCatalogCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Cargo
    )

    foreach ($caseDir in @(
        "benchmarks\B01_csv_heat_rate",
        "benchmarks\B02_timeseries_fusion",
        "benchmarks\B03_state_space",
        "benchmarks\B04_residual_eval",
        "benchmarks\B05_component_solver",
        "benchmarks\B06_nonlinear_solver"
    )) {
        Invoke-JitBenchmarkCaseCheck $Cargo $caseDir
    }
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
    Invoke-JitBenchTargetCheck $cargo "examples\official\01_csv_plot\main.eng" "csv_heat_rate_workflow" "covered_by_current_source" "timeseries_integrate"
    Invoke-JitBenchTargetCheck $cargo "examples\official\01_csv_plot\main.eng" "multi_statistics_fusion" "covered_by_current_source" "statistics_fusion:summary:Q_coil"
    Invoke-JitBenchTargetCheck $cargo "examples\internal\21_thermal_component_assembly\main.eng" "residual_evaluation" "covered_by_current_source" "component_residual_jacobian"
    Invoke-JitBenchTargetCheck $cargo "examples\internal\21_thermal_component_assembly\main.eng" "component_graph_solver_small_case" "covered_by_current_source" "component_newton_step"
    Invoke-JitBenchTargetCheck $cargo "examples\internal\18_state_space_metadata\main.eng" "state_space_simulation" "covered_by_current_source" "state_space_solver_step"
    Invoke-JitBenchmarkCatalogCheck $cargo
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

function Copy-IdeRuntimeDependencies {
    param(
        [Parameter(Mandatory = $true)]
        [string] $DestinationRoot
    )

    $WebView2Loader = Join-Path $RepoRoot "target\release\WebView2Loader.dll"
    if (-not (Test-Path $WebView2Loader)) {
        throw "missing WebView2Loader.dll at $WebView2Loader; build eng_ide in release mode first"
    }
    Copy-Item -Force $WebView2Loader (Join-Path $DestinationRoot "WebView2Loader.dll")
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
    Copy-IdeRuntimeDependencies -DestinationRoot $CurrentRoot
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
    $WebView2LoaderHash = (Get-FileHash -Algorithm SHA256 (Join-Path $CurrentRoot "WebView2Loader.dll")).Hash.ToLowerInvariant()
    $LspHash = (Get-FileHash -Algorithm SHA256 (Join-Path $CurrentRoot "eng-lsp.exe")).Hash.ToLowerInvariant()

    Set-Content -Path (Join-Path $CurrentRoot "README.txt") -Encoding ascii -Value @"
EngLang dev-current

This folder is the current commit's non-release test display build.
Run eng-ide.exe from this folder to open the Tauri/WebView IDE with bundled examples,
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
webview2_loader_sha256 = $WebView2LoaderHash
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
    <Description xml:space="preserve">EngLang editor tooling with diagnostics, hover, completion, and run commands.</Description>
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

    Assert-VscodeExtensionContract
    $Version = Get-WorkspaceVersion
    $ExtensionSource = Join-Path $RepoRoot "tools\vscode-englang"
    $ToolsRoot = Join-Path $PackageRoot "tools"
    $ExtensionOut = Join-Path $ToolsRoot "vscode-englang"
    $VsixStage = Join-Path $RepoRoot "build\vscode-vsix"
    $VsixExtensionRoot = Join-Path $VsixStage "extension"
    $VsixPath = Join-Path $ToolsRoot (Get-VsixFileName)
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

function Get-VscodeCli {
    $CodeCmd = Get-Command "code.cmd" -ErrorAction SilentlyContinue
    if ($null -ne $CodeCmd) {
        return $CodeCmd.Source
    }
    $Code = Get-Command "code" -ErrorAction SilentlyContinue
    if ($null -ne $Code) {
        return $Code.Source
    }
    return $null
}

function Invoke-VscodePackage {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "--release" "-p" "eng_cli" "-p" "eng_lsp"
    $PackageRoot = Join-Path $RepoRoot "dist\local-vscode"
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    Invoke-IdePackage -PackageRoot $PackageRoot
    $VsixPath = Join-Path (Join-Path $PackageRoot "tools") (Get-VsixFileName)
    Write-Host "Local VS Code VSIX ready: $VsixPath"
    return $VsixPath
}

function Invoke-VscodeInstall {
    $VsixPath = Invoke-VscodePackage
    $Code = Get-VscodeCli
    if ($null -eq $Code) {
        throw "VS Code CLI not found. Install the VSIX manually from $VsixPath, or add the VS Code 'code' command to PATH."
    }
    Invoke-Native $Code "--install-extension" $VsixPath "--force"
    Write-Host "Installed EngLang VS Code extension from $VsixPath"
    Write-Host "Reload VS Code windows that already had EngLang files open."
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

function Invoke-UserDocsMarkdown {
    Set-DevEnvironment
    $python = Get-PortablePython
    $script = Join-Path $RepoRoot "docs\user\build_user_docs.py"
    if ($null -eq $python -or -not (Test-Path -LiteralPath $script)) {
        throw "user docs Markdown assembly requires portable Python from .\dev.bat setup"
    }
    Invoke-Native $python $script "--assemble-markdown"
}

function New-UserGuidePdf {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $Version
    )

    $sections = @(
        @{ Kind = "title"; Text = "EngLang User Guide" },
        @{ Kind = "subtitle"; Text = "Portable Windows package v$Version" },
        @{ Kind = "body"; Text = "EngLang is a native engineering language for workflows where units, physical quantities, schemas, axes, statistics, plots, reports, and provenance are checked as part of the program. This PDF is the curated user-facing guide for the portable package; developer notes and master plans stay in the repository." },
        @{ Kind = "h1"; Text = "1. Package Contents" },
        @{ Kind = "body"; Text = "The portable folder contains eng.exe for command-line execution, eng-ide.exe for native IDE testing, eng-lsp.exe for editor tooling checks, WebView2Loader.dll for the IDE, official core workflow examples, standard library source files, optional VS Code extension tooling, curated PDF docs, README.txt, and PACKAGE_ASSETS.txt. Advanced solver, compatibility, diagnostic, and internal regression fixtures stay in the source repository. The package intentionally does not ship the full developer documentation tree." },
        @{ Kind = "h1"; Text = "2. First Smoke Test" },
        @{ Kind = "step"; Text = "Open a command prompt in the extracted folder." },
        @{ Kind = "step"; Text = "Run: eng.exe doctor" },
        @{ Kind = "step"; Text = "Run: eng-ide.exe --smoke" },
        @{ Kind = "step"; Text = "Run: eng-lsp.exe --smoke" },
        @{ Kind = "body"; Text = "All three commands should exit successfully. The doctor command verifies runtime, standard library, unit registry, plot renderer, report generator, write permission, and example files. The IDE smoke command verifies that examples and compiler completion metadata are discoverable. The editor tooling smoke command verifies diagnostics, completion, and hover metadata for editor integrations." },
        @{ Kind = "h1"; Text = "3. Tauri IDE Workflow" },
        @{ Kind = "step"; Text = "Run: eng-ide.exe" },
        @{ Kind = "step"; Text = "Use Explorer to open examples/official/01_csv_plot/main.eng or create a scratch .eng file." },
        @{ Kind = "step"; Text = "Use Check for lint diagnostics. Error and warning counts are visible in the toolbar and details are listed in Problems." },
        @{ Kind = "step"; Text = "Use Ctrl+Space in the editor to open caret completions, then insert symbols, keywords, quantity kinds, units, or snippets." },
        @{ Kind = "step"; Text = "Use Run to execute the current top-level file. The IDE updates the terminal, Problems tab, Variables table, and PlotSpec preview." },
        @{ Kind = "body"; Text = "The IDE uses the same compiler and runtime crates as eng.exe. Diagnostics, symbols, completions, run artifacts, and report generation therefore test the real core path rather than duplicated editor logic." },
        @{ Kind = "h1"; Text = "4. CSV Plot Example" },
        @{ Kind = "body"; Text = "The CSV plot example is the recommended user test because one file exercises typed CSV promotion, unit-aware HeatRate calculations, TimeSeries statistics, integration metadata, PlotSpec/SVG output, report output, and standalone packaging without presenting solver support as a public release claim." },
        @{ Kind = "body"; Text = "From the command line, run: eng.exe run examples/official/01_csv_plot/main.eng --save-artifacts" },
        @{ Kind = "h1"; Text = "5. Expected Output" },
        @{ Kind = "body"; Text = "After a successful run, inspect build/result/report.html first. The result folder also contains result.engres, review.json, report_spec.json, plots/plot_spec.json, plots/plot_manifest.json, and plots/timeseries.svg." },
        @{ Kind = "body"; Text = "The result should record typed CSV provenance, computed statistics, an integration result, PlotSpec/SVG metadata, and report artifacts." },
        @{ Kind = "h1"; Text = "6. Useful User Edits" },
        @{ Kind = "step"; Text = "Change the plot title and run again to verify report regeneration." },
        @{ Kind = "step"; Text = "Change duration_above(5 kW) to duration_above(4.5 kW) and compare computed statistics." },
        @{ Kind = "step"; Text = "Temporarily change m_dot <= 0.30 kg/s to m_dot <= 0.20 kg/s and inspect policy results." },
        @{ Kind = "step"; Text = "Type Heat and use completion to insert HeatRate or HeatCapacity." },
        @{ Kind = "h1"; Text = "7. Troubleshooting" },
        @{ Kind = "body"; Text = "If a run fails, check Problems first, then run eng.exe check <file.eng> from the same folder. Plot previews live in the right Plot inspector tab beside Variables; report and SVG artifacts can be opened on demand after a successful run." },
        @{ Kind = "body"; Text = "If a CSV path fails, keep relative paths anchored next to the source file, as in the official examples. If a report does not open, open build/result/report.html manually." },
        @{ Kind = "h1"; Text = "8. Current Boundaries" },
        @{ Kind = "body"; Text = "This release supports the documented core workflows: CSV promote, unit-aware TimeSeries calculations, PlotSpec/SVG output, review/report artifacts, package smoke, official examples, standalone packaging, and the native tester IDE smoke path. Advanced solver, compatibility, diagnostic, and internal regression fixtures remain source-repository material, not portable package tutorials. Public solver support, general nonlinear solving, DAE solving, production multi-domain component graph solving, native JIT execution, broad uncertainty and ML workflows, and full editor platform guarantees remain future or internal tracks unless explicitly marked stable-supported." }
    )

    $pages = New-Object System.Collections.Generic.List[string]
    $content = New-Object System.Collections.Generic.List[string]
    $script:EngPdfY = 740
    $script:EngPdfPageNumber = 1

    function Add-PdfPage {
        if ($content.Count -gt 0) {
            $content.Add("BT /F1 8 Tf 54 34 Td (EngLang v$Version user guide - page $script:EngPdfPageNumber) Tj ET") | Out-Null
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

function New-PackageAssetManifest {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $PublicVersion
    )

    Set-Content -Path $Path -Encoding ascii -Value @"
EngLang portable package assets

public_version = $PublicVersion
generated_by = dev.bat package

Runtime binaries:
  eng.exe                         command-line checker, runner, viewer, formatter, and packager
  eng-ide.exe                     native Tauri/WebView IDE for local testing and inspection
  eng-lsp.exe                     language-server binary used by editor tooling and smoke checks
  WebView2Loader.dll              IDE runtime dependency that must stay next to eng-ide.exe

Curated user documentation:
  docs\$(Get-PackageUserGuideFileName)
  docs\EngLang_Language_Grammar_Guide.pdf

Examples:
  examples\official\             core workflow examples and user tests

Repo-only examples excluded from this portable package:
  advanced solver smoke fixtures
  compatibility regression fixtures
  internal smoke and inspection fixtures
  diagnostic and data-quality fixtures

Language support:
  stdlib\                         packaged standard library source files

Optional editor tooling:
  tools\vscode-englang\           VS Code extension source
  tools\$(Get-VsixFileName)       installable VS Code extension package

Package start files:
  README.txt                      short start page and smoke commands
  PACKAGE_ASSETS.txt              this portable asset inventory

Intentionally excluded:
  developer markdown documentation tree
  local .dev toolchain cache
  target, build, and dist history
  Rust, Python, Node, and Visual Studio Build Tools target dependencies
"@
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
    $PackageRoot = Join-Path $RepoRoot ("dist\" + (Get-PackageRootName))
    $ZipPath = Join-Path $RepoRoot ("dist\" + (Get-ZipFileName))
    $ChecksumPath = "$ZipPath.sha256"
    $ReleaseGuidePath = Join-Path $RepoRoot ("dist\" + (Get-UserGuideFileName))
    Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ZipPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ReleaseGuidePath -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng.exe") (Join-Path $PackageRoot "eng.exe")
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng-ide.exe") (Join-Path $PackageRoot "eng-ide.exe")
    Copy-Item -Force (Join-Path $RepoRoot "target\release\eng-lsp.exe") (Join-Path $PackageRoot "eng-lsp.exe")
    Copy-IdeRuntimeDependencies -DestinationRoot $PackageRoot
    $PackageExamplesRoot = Join-Path $PackageRoot "examples"
    New-Item -ItemType Directory -Force -Path $PackageExamplesRoot | Out-Null
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "examples\official") (Join-Path $PackageExamplesRoot "official")
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "stdlib") (Join-Path $PackageRoot "stdlib")
    New-Item -ItemType Directory -Force -Path (Join-Path $PackageRoot "docs") | Out-Null
    $PackageGuidePath = Join-Path $PackageRoot ("docs\" + (Get-PackageUserGuideFileName))
    if (-not (New-UserGuideWithOodocs -Path $PackageGuidePath -Version $PublicVersion)) {
        New-UserGuidePdf -Path $PackageGuidePath -Version $PublicVersion
    }
    $PackageGrammarGuidePath = Join-Path $PackageRoot "docs\EngLang_Language_Grammar_Guide.pdf"
    if (-not (New-GrammarGuideWithOodocs -Path $PackageGrammarGuidePath -Version $PublicVersion)) {
        throw "Could not generate EngLang language grammar guide with OODocs."
    }
    Copy-Item -Force $PackageGuidePath $ReleaseGuidePath
    Invoke-IdePackage -PackageRoot $PackageRoot
    New-PackageAssetManifest -Path (Join-Path $PackageRoot "PACKAGE_ASSETS.txt") -PublicVersion $PublicVersion
    Set-Content -Path (Join-Path $PackageRoot "README.txt") -Encoding ascii -Value @"
EngLang portable package

This folder is self-contained for EngLang execution. Rust and Python are not
required on the target PC. WebView2Loader.dll is bundled next to eng-ide.exe
for the Tauri/WebView IDE.

Start here:
  docs\$(Get-PackageUserGuideFileName)
  docs\EngLang_Language_Grammar_Guide.pdf
  PACKAGE_ASSETS.txt

Recommended smoke commands:
  eng.exe doctor
  eng-ide.exe --smoke
  eng-lsp.exe --smoke
  eng-ide.exe
  eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts
  eng.exe run examples\official\09_command_where_with\main.eng --save-artifacts
  eng.exe run examples\official\16_test_assert_golden\main.eng --save-artifacts
  eng.exe build examples\official\01_csv_plot\main.eng --standalone --profile repro
  dist\main-standalone\run.bat --help
  dist\main-standalone\run.bat
  eng.exe view build\result\result.engres

VS Code IDE preview:
  code --install-extension tools\$(Get-VsixFileName)
  open a .eng file
  run "EngLang: Check Current File"

Example folders:
  examples\official      core workflow release examples

Repo-only example folders not bundled in this package:
  advanced solver smoke fixtures
  compatibility regression fixtures
  internal smoke and inspection fixtures
  diagnostic and data-quality fixtures

Generated artifacts are written under build\result in the current folder.
The curated user guide is docs\$(Get-PackageUserGuideFileName). The language
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
    $ZipPath = Join-Path $RepoRoot ("dist\" + (Get-ZipFileName))
    $KoreanWord = -join @([char]0xD55C, [char]0xAE00)
    $SmokeRoot = Join-Path $RepoRoot "dist\portable smoke $KoreanWord"
    Remove-Item -LiteralPath $SmokeRoot -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $SmokeRoot | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $SmokeRoot -Force
    $Eng = Join-Path $SmokeRoot "eng.exe"
    $Lsp = Join-Path $SmokeRoot "eng-lsp.exe"
    $WebView2Loader = Join-Path $SmokeRoot "WebView2Loader.dll"
    if (-not (Test-Path $WebView2Loader)) {
        throw "portable smoke missing WebView2Loader.dll next to eng-ide.exe"
    }

    Push-Location $SmokeRoot
    try {
        Invoke-Native $Eng "doctor"
        Invoke-Native (Join-Path $SmokeRoot "eng-ide.exe") "--smoke"
        Invoke-Native $Lsp "--smoke"
        Invoke-Native $Eng "run" "examples\official\01_csv_plot\main.eng" "--save-artifacts"
        Invoke-Native $Eng "view" "build\result\result.engres"
        Invoke-Native $Eng "run" "examples\official\09_command_where_with\main.eng" "--save-artifacts"
        if (-not (Test-Path (Join-Path $SmokeRoot "build\result\report_spec.json"))) {
            throw "portable smoke did not create build\result\report_spec.json"
        }
        Invoke-Native $Eng "run" "examples\official\16_test_assert_golden\main.eng" "--save-artifacts"
        $ReleaseTestResultsPath = Join-Path $SmokeRoot "build\result\test_results.json"
        if (-not (Test-Path $ReleaseTestResultsPath)) {
            throw "portable smoke official examples did not create test_results.json"
        }
        $ReleaseSmokeTests = Get-Content -LiteralPath $ReleaseTestResultsPath -Raw
        if (-not $ReleaseSmokeTests.Contains('"status"')) {
            throw "portable smoke official examples did not produce expected release artifacts"
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
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\completionProvider.js"))) {
            throw "portable package did not include VS Code completion provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\localCodeActions.js"))) {
            throw "portable package did not include VS Code local quick fix provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\lspCodeActions.js"))) {
            throw "portable package did not include VS Code LSP quick fix bridge"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\lspKinds.js"))) {
            throw "portable package did not include VS Code LSP kind bridge"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\lspNavigation.js"))) {
            throw "portable package did not include VS Code LSP navigation bridge"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\lspRanges.js"))) {
            throw "portable package did not include VS Code LSP range bridge"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\lspSemanticTokens.js"))) {
            throw "portable package did not include VS Code LSP semantic token bridge"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\artifactRegistry.js"))) {
            throw "portable package did not include VS Code artifact registry"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\editorMetadata.js"))) {
            throw "portable package did not include VS Code editor metadata loader"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\executionProfiles.js"))) {
            throw "portable package did not include VS Code execution profiles registry"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\moduleStatus.js"))) {
            throw "portable package did not include VS Code module status wording registry"
        }
        if (-not (Test-Path $Lsp)) {
            throw "portable package did not include eng-lsp.exe"
        }
        $ExpectedVsix = Join-Path $SmokeRoot ("tools\" + (Get-VsixFileName))
        if (-not (Test-Path $ExpectedVsix)) {
            throw "portable package did not include VS Code VSIX"
        }
        $ExpectedUserGuide = Join-Path $SmokeRoot ("docs\" + (Get-PackageUserGuideFileName))
        if (-not (Test-Path $ExpectedUserGuide)) {
            throw "portable package did not include user guide PDF"
        }
        if ((Get-Item -LiteralPath $ExpectedUserGuide).Length -lt 20000) {
            throw "portable package user guide PDF is unexpectedly small"
        }
        $ExpectedGrammarGuide = Join-Path $SmokeRoot "docs\EngLang_Language_Grammar_Guide.pdf"
        if (-not (Test-Path $ExpectedGrammarGuide)) {
            throw "portable package did not include language grammar guide PDF"
        }
        if ((Get-Item -LiteralPath $ExpectedGrammarGuide).Length -lt 20000) {
            throw "portable package language grammar guide PDF is unexpectedly small"
        }
        $ExpectedAssetManifest = Join-Path $SmokeRoot "PACKAGE_ASSETS.txt"
        if (-not (Test-Path $ExpectedAssetManifest)) {
            throw "portable package did not include PACKAGE_ASSETS.txt"
        }
        $AssetManifestText = Get-Content -LiteralPath $ExpectedAssetManifest -Raw -Encoding UTF8
        if (-not $AssetManifestText.Contains("examples\official\") -or -not $AssetManifestText.Contains("Repo-only examples excluded")) {
            throw "portable package asset manifest does not document example support boundaries"
        }
        foreach ($ExcludedExampleFolder in @("advanced_solver", "compat", "internal", "diagnostics")) {
            if (Test-Path (Join-Path $SmokeRoot "examples\$ExcludedExampleFolder")) {
                throw "portable package should not include examples\$ExcludedExampleFolder"
            }
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
    $ZipPath = Join-Path $RepoRoot ("dist\" + (Get-ZipFileName))
    $ChecksumPath = "$ZipPath.sha256"
    if (-not (Test-Path $ZipPath)) {
        throw "release check did not create $ZipPath"
    }
    if (-not (Test-Path $ChecksumPath)) {
        throw "release check did not create $ChecksumPath"
    }
    $GuidePath = Join-Path $RepoRoot ("dist\" + (Get-UserGuideFileName))
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
  .\dev.bat workflows-test Run native workflow module and workflow example smoke tests
  .\dev.bat fmt            Format Rust code
  .\dev.bat clippy         Run clippy with warnings denied
  .\dev.bat ci             Run fmt, tests, clippy, and preview example
  .\dev.bat docs-check     Check supported documentation Eng snippets
  .\dev.bat user-docs-markdown Assemble curated user guide Markdown for publishing checks
  .\dev.bat grammar-docs   Generate the oodocs language grammar PDF
  .\dev.bat vscode-build-grammar Regenerate VS Code TextMate grammar from source JSON
  .\dev.bat vscode-build-editor-metadata Regenerate VS Code editor metadata from eng-lsp
  .\dev.bat vscode-grammar-test  Check VS Code TextMate grammar source, generated output, and token fixtures
  .\dev.bat vscode-package Build a local installable VS Code extension VSIX
  .\dev.bat vscode-install Build and install the EngLang VS Code extension with the code CLI
  .\dev.bat ide-check      Validate the Tauri IDE and VS Code extension preview
  .\dev.bat lsp-check      Validate eng-lsp.exe stdio, smoke, and snapshot output
  .\dev.bat jit-check      Validate runtime optimization track kernel planning and bench output
  .\dev.bat ide            Run the Tauri/WebView EngLang tester IDE
  .\dev.bat dev-current    Build latest release test IDE into dist\dev-current
  .\dev.bat artifacts-check Validate artifact schemas and golden baselines
  .\dev.bat run-example    Run examples\official\01_csv_plot\main.eng
  .\dev.bat package        Build release, assemble dist package folder, zip it, and write SHA256
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
    "workflows-test" { Invoke-WorkflowsTest }
    "fmt" { Invoke-Fmt }
    "clippy" { Invoke-Clippy }
    "ci" { Invoke-Ci }
    "docs-check" { Invoke-DocsCheck }
    "user-docs-markdown" { Invoke-UserDocsMarkdown }
    "grammar-docs" { Invoke-GrammarDocs }
    "vscode-build-grammar" { Invoke-VscodeBuildGrammar }
    "vscode-build-editor-metadata" { Invoke-VscodeBuildEditorMetadata }
    "vscode-grammar-test" { Invoke-VscodeGrammarTest }
    "vscode-package" { Invoke-VscodePackage }
    "vscode-install" { Invoke-VscodeInstall }
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
