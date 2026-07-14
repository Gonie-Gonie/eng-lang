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

function Invoke-NativeInDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string] $WorkingDirectory,

        [Parameter(Mandatory = $true)]
        [string] $FilePath,

        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]] $Arguments
    )

    New-Item -ItemType Directory -Force -Path $WorkingDirectory | Out-Null
    Push-Location -LiteralPath $WorkingDirectory
    try {
        Invoke-Native $FilePath @Arguments
    } finally {
        Pop-Location
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
    Write-Host "The native IDE uses static HTML/CSS/JS assets; Node/npm is not required."
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
        "generated output list",
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
    $RequiredWorkflowSourcePaths = @(
        (Join-Path $WorkflowRoot "01_weather_api_to_standard_file\main.eng"),
        (Join-Path $WorkflowRoot "02_native_surrogate_case_workflow\main.eng"),
        (Join-Path $WorkflowRoot "03_uncertain_sensor_report\main.eng")
    )
    foreach ($RequiredWorkflowSourcePath in $RequiredWorkflowSourcePaths) {
        if (-not (Test-Path -LiteralPath $RequiredWorkflowSourcePath -PathType Leaf)) {
            throw "Native workflow smoke missing required workflow entrypoint: $RequiredWorkflowSourcePath"
        }
    }
    $ForbiddenNativeWorkflowSourceMarkers = @(
        "\bpython(?:\d+(?:\.\d+)*)?(?:\.exe)?\b",
        "\bpy(?:\.exe)?\b",
        "\.py\b",
        "\.pyw\b",
        "\.ipynb\b",
        "\bpip(?:3)?\b",
        "\bconda\b",
        "\bpoetry\b",
        "\bpyenv\b",
        "\bmamba\b",
        "\bmicromamba\b",
        "\bvirtualenv\b",
        "\bvenv\b",
        "\bipython\b",
        "\bpytest\b",
        "\btox\b",
        "\bnox\b",
        "\bmypy\b",
        "\bruff\b",
        "\bsubprocess\b",
        "\bpandas\b",
        "\bnumpy\b",
        "\bscipy\b",
        "\bsklearn\b",
        "\bstatsmodels\b",
        "\bpolars\b",
        "\bmatplotlib\b",
        "\brequests\b",
        "\burllib\b",
        "\bpyarrow\b",
        "\bxarray\b",
        "\btensorflow\b",
        "\bpytorch\b",
        "\btorch\b",
        "\bjupyter\b",
        "\bjupyterlab\b",
        "\bnotebook\b"
    )
    $ForbiddenNativeWorkflowDocMarkers = @(
        "\bpython(?:\d+(?:\.\d+)*)?(?:\.exe)?\b",
        "\bpy(?:\.exe)?\b",
        "\.py\b",
        "\.pyw\b",
        "\.ipynb\b",
        "\bpip(?:3)?\b",
        "\bconda\b",
        "\bpoetry\b",
        "\bpyenv\b",
        "\bmamba\b",
        "\bmicromamba\b",
        "\bvirtualenv\b",
        "\bvenv\b",
        "\bipython\b",
        "\bpytest\b",
        "\btox\b",
        "\bnox\b",
        "\bmypy\b",
        "\bruff\b",
        "\bsubprocess\b",
        "\bpandas\b",
        "\bnumpy\b",
        "\bscipy\b",
        "\bsklearn\b",
        "\bstatsmodels\b",
        "\bpolars\b",
        "\bmatplotlib\b",
        "\bpyarrow\b",
        "\bxarray\b",
        "\btensorflow\b",
        "\bpytorch\b",
        "\btorch\b",
        "\bjupyter\b",
        "\bjupyterlab\b",
        "\bnotebook\b"
    )
    $NativeWorkflowSourceAuditPaths = @($RequiredWorkflowSourcePaths | ForEach-Object {
        Split-Path -Parent $_
    } | Sort-Object -Unique | ForEach-Object {
        Get-ChildItem -LiteralPath $_ -Recurse -File -Filter "*.eng"
    } | ForEach-Object {
        $_.FullName
    } | Sort-Object -Unique)
    foreach ($NativeWorkflowSourceAuditPath in $NativeWorkflowSourceAuditPaths) {
        $Workflow = $NativeWorkflowSourceAuditPath.Substring($RepoRoot.Length).TrimStart('\')
        $WorkflowSource = Get-Content -LiteralPath $NativeWorkflowSourceAuditPath -Raw
        if ($WorkflowSource -match "(?im)\brun\s+command\b") {
            throw "Native workflow source must not use run command: $Workflow"
        }
        foreach ($PythonMarker in $ForbiddenNativeWorkflowSourceMarkers) {
            if ($WorkflowSource -match "(?i)$PythonMarker") {
                throw "Native workflow source must not contain Python/notebook marker $PythonMarker`: $Workflow"
            }
        }
        if ($WorkflowSource -match "(?i)\bselect_first_row\s*\(") {
            throw "Native workflow source must use filter + require_one instead of legacy select_first_row: $Workflow"
        }
    }
    $WorkflowPublicDocPaths = @(
        @(Get-ChildItem -LiteralPath $WorkflowRoot -Recurse -File -Include "*.md", "*.txt" | Sort-Object FullName)
        @(Get-ChildItem -LiteralPath (Join-Path $RepoRoot "docs\workflows") -File -Filter "*.md" | Sort-Object FullName)
        @(
            "examples\README.md",
            "docs\user\tutorial\12_composite_workflow.md",
            "docs\current\workflow_modules.md",
            "docs\current\test_ci_gates.md"
        ) | ForEach-Object {
            Get-Item -LiteralPath (Join-Path $RepoRoot $_)
        }
    )
    foreach ($WorkflowPublicDocPath in $WorkflowPublicDocPaths) {
        $WorkflowPublicDoc = Get-Content -LiteralPath $WorkflowPublicDocPath.FullName -Raw
        if ($WorkflowPublicDoc -match "(?im)\brun\s+command\b") {
            throw "Native workflow public docs must not describe workflow 01/02/03 as run-command backed: $($WorkflowPublicDocPath.FullName)"
        }
        foreach ($PythonMarker in $ForbiddenNativeWorkflowDocMarkers) {
            if ($WorkflowPublicDoc -match "(?i)$PythonMarker") {
                throw "Native workflow public docs must not contain Python/notebook marker $PythonMarker`: $($WorkflowPublicDocPath.FullName)"
            }
        }
        foreach ($ForbiddenWorkflowDocWording in @(
            "files produced by an external process",
            "external-simulator adapter pattern",
            "native surrogate half",
            "external simulator adapter could feed later",
            "Python process:",
            "created by Python",
            "Python-created",
            "generated by Python",
            "Python-generated",
            "Python-made",
            "Python-backed",
            "Python-side",
            "CSV fixture",
            "02_external_simulation_surrogate",
            "external_simulation_surrogate.md"
        )) {
            if ($WorkflowPublicDoc.Contains($ForbiddenWorkflowDocWording)) {
                throw "Native workflow public docs must not lead with stale external-process wording '$ForbiddenWorkflowDocWording': $($WorkflowPublicDocPath.FullName)"
            }
        }
    }
    Write-Host "Native workflow Python/process guard passed. Checked $(@($NativeWorkflowSourceAuditPaths).Count) source file(s) and $(@($WorkflowPublicDocPaths).Count) public doc file(s) for Python/notebook/run-command markers."

    foreach ($WorkflowSourcePath in $WorkflowSourcePaths) {
        $Workflow = $WorkflowSourcePath.Substring($RepoRoot.Length).TrimStart('\')
        $WorkflowSource = Get-Content -LiteralPath $WorkflowSourcePath -Raw
        if ($WorkflowSource -match "(?im)\brun\s+command\b") {
            throw "Native workflow source must not use run command: $Workflow"
        }
        foreach ($PythonMarker in $ForbiddenNativeWorkflowSourceMarkers) {
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
                    "offline_response_file",
                    "schema StationMap with two fixture rows",
                    'This checked workflow keeps `offline_response` enabled'
                )) {
                    if ($WorkflowPublicText.Contains($ForbiddenWorkflowWording)) {
                        throw "Workflow 01 public wording should describe pinned offline responses instead of '$ForbiddenWorkflowWording': $WorkflowPublicTextPath"
                    }
                }
                if ($WorkflowPublicTextPath -like "*01_weather_api_to_standard_file*README.md" -and -not $WorkflowPublicText.Contains('`args.pinned_response_file` feeds the language-level `offline_response`')) {
                    throw "Workflow 01 README must distinguish the public pinned_response_file arg from the language-level offline_response option: $WorkflowPublicTextPath"
                }
            }
        }
        Invoke-Native $cargo "run" "-p" "eng_cli" "--" "run" $Workflow "--save-artifacts"
        $ProcessResultsPath = Join-Path $RepoRoot "build\result\process_results.json"
        if (-not (Test-Path -LiteralPath $ProcessResultsPath)) {
            throw "Native workflow smoke must write process_results.json: $Workflow"
        }
        $ProcessResults = Get-Content -LiteralPath $ProcessResultsPath -Raw | ConvertFrom-Json
        if ([string]$ProcessResults.format -ne "eng-process-results-v1") {
            throw "Native workflow smoke must write eng-process-results-v1 process results: $Workflow"
        }
        if ([string]$ProcessResults.execution_profile -ne "normal") {
            throw "Native workflow smoke must record the normal execution profile in process_results.json: $Workflow"
        }
        $ProcessCount = 0
        if ($null -ne $ProcessResults.process_count) {
            $ProcessCount = [int]$ProcessResults.process_count
        }
        if ($null -eq $ProcessResults.processes) {
            throw "Native workflow smoke must record an empty processes array: $Workflow"
        }
        $ProcessListCount = 0
        if ($null -ne $ProcessResults.processes) {
            $ProcessListCount = @($ProcessResults.processes).Count
        }
        if ($ProcessCount -ne 0 -or $ProcessListCount -ne 0) {
            throw "Native workflow smoke must not execute external processes: $Workflow"
        }
        foreach ($NativeWorkflowArtifactTextPath in @(
            (Join-Path $RepoRoot "build\result\result.engres"),
            (Join-Path $RepoRoot "build\result\review.json"),
            (Join-Path $RepoRoot "build\result\output_manifest.json"),
            (Join-Path $RepoRoot "build\result\run_log.json"),
            (Join-Path $RepoRoot "build\result\run_lock.json"),
            (Join-Path $RepoRoot "build\result\static_run_plan.json"),
            (Join-Path $RepoRoot "build\result\run_plan.json"),
            (Join-Path $RepoRoot "build\result\cache_manifest.json"),
            (Join-Path $RepoRoot "build\result\report_spec.json")
        )) {
            if (-not (Test-Path -LiteralPath $NativeWorkflowArtifactTextPath -PathType Leaf)) {
                continue
            }
            $NativeWorkflowArtifactText = Get-Content -LiteralPath $NativeWorkflowArtifactTextPath -Raw
            if ($NativeWorkflowArtifactText -match "(?im)\brun\s+command\b") {
                throw "Native workflow artifact must not contain run-command wording: $Workflow -> $NativeWorkflowArtifactTextPath"
            }
            foreach ($PythonMarker in $ForbiddenNativeWorkflowSourceMarkers) {
                if ($NativeWorkflowArtifactText -match "(?i)$PythonMarker") {
                    throw "Native workflow artifact must not contain Python/notebook marker $PythonMarker`: $Workflow -> $NativeWorkflowArtifactTextPath"
                }
            }
        }
        foreach ($NativeWorkflowRunGraphPath in @(
            (Join-Path $RepoRoot "build\result\static_run_plan.json"),
            (Join-Path $RepoRoot "build\result\run_plan.json")
        )) {
            if (-not (Test-Path -LiteralPath $NativeWorkflowRunGraphPath -PathType Leaf)) {
                throw "Native workflow smoke missing run graph artifact: $NativeWorkflowRunGraphPath"
            }
            $NativeWorkflowRunGraph = Get-Content -LiteralPath $NativeWorkflowRunGraphPath -Raw | ConvertFrom-Json
            foreach ($NativeWorkflowRunGraphNode in @($NativeWorkflowRunGraph.graph.nodes)) {
                foreach ($NativeWorkflowRunGraphField in @(
                    [string]$NativeWorkflowRunGraphNode.id,
                    [string]$NativeWorkflowRunGraphNode.kind,
                    [string]$NativeWorkflowRunGraphNode.label
                )) {
                    if ($NativeWorkflowRunGraphField -match "(?i)^process:" -or $NativeWorkflowRunGraphField -match "(?i)\brun\s+command\b") {
                        throw "Native workflow run graph must not contain process/run-command node metadata '$NativeWorkflowRunGraphField': $Workflow"
                    }
                    foreach ($PythonMarker in $ForbiddenNativeWorkflowSourceMarkers) {
                        if ($NativeWorkflowRunGraphField -match "(?i)$PythonMarker") {
                            throw "Native workflow run graph must not contain Python/notebook marker $PythonMarker in '$NativeWorkflowRunGraphField': $Workflow"
                        }
                    }
                }
            }
            foreach ($NativeWorkflowRunGraphEdge in @($NativeWorkflowRunGraph.graph.edges)) {
                foreach ($NativeWorkflowRunGraphField in @(
                    [string]$NativeWorkflowRunGraphEdge.from,
                    [string]$NativeWorkflowRunGraphEdge.to,
                    [string]$NativeWorkflowRunGraphEdge.kind
                )) {
                    if ($NativeWorkflowRunGraphField -match "(?i)^process:" -or $NativeWorkflowRunGraphField -match "(?i)\brun\s+command\b") {
                        throw "Native workflow run graph must not contain process/run-command edge metadata '$NativeWorkflowRunGraphField': $Workflow"
                    }
                    foreach ($PythonMarker in $ForbiddenNativeWorkflowSourceMarkers) {
                        if ($NativeWorkflowRunGraphField -match "(?i)$PythonMarker") {
                            throw "Native workflow run graph must not contain Python/notebook marker $PythonMarker in '$NativeWorkflowRunGraphField': $Workflow"
                        }
                    }
                }
            }
        }
        if ($Workflow -like "*01_weather_api_to_standard_file*") {
            $ResultPath = Join-Path $RepoRoot "build\result\result.engres"
            $ReviewPath = Join-Path $RepoRoot "build\result\review.json"
            $OutputManifestPath = Join-Path $RepoRoot "build\result\output_manifest.json"
            $CacheManifestPath = Join-Path $RepoRoot "build\result\cache_manifest.json"
            $RunLogPath = Join-Path $RepoRoot "build\result\run_log.json"
            foreach ($RequiredWorkflowArtifactPath in @($ResultPath, $ReviewPath, $OutputManifestPath, $CacheManifestPath, $RunLogPath)) {
                if (-not (Test-Path -LiteralPath $RequiredWorkflowArtifactPath)) {
                    throw "Workflow 01 native contract smoke missing artifact: $RequiredWorkflowArtifactPath"
                }
            }
            $ResultJson = Get-Content -LiteralPath $ResultPath -Raw
            $ReviewJson = Get-Content -LiteralPath $ReviewPath -Raw
            $OutputManifestJson = Get-Content -LiteralPath $OutputManifestPath -Raw
            $CacheManifestJson = Get-Content -LiteralPath $CacheManifestPath -Raw
            $RunLogJson = Get-Content -LiteralPath $RunLogPath -Raw
            $ResultData = $ResultJson | ConvertFrom-Json
            $ReviewData = $ReviewJson | ConvertFrom-Json
            $OutputManifestData = $OutputManifestJson | ConvertFrom-Json
            $CacheManifestData = $CacheManifestJson | ConvertFrom-Json
            $RunLogData = $RunLogJson | ConvertFrom-Json
            $WeatherResponseHash = "d7960daaab0788c185af699f9372660383e8a41cb1db1e8a020f75db80f5feff"
            $AllowedCacheStatuses = @("hit", "miss_offline_response_available", "offline_response_available", "miss_materialized")
            $AllowedCacheStatusDescription = "hit, offline-response available, or materialized miss"
            if (-not $WorkflowSource.Contains("pinned_response_file") -or $WorkflowSource.Contains("offline_response_file")) {
                throw "Workflow 01 native API args must expose pinned_response_file and not offline_response_file"
            }
            Assert-ArtifactNumber $ResultData.provenance.network_boundary_count 1 "Workflow 01 result network boundary count"
            $NetworkBoundary = @($ResultData.typed_payload.network_boundaries | Where-Object { [string]$_.binding -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $NetworkBoundary) "Workflow 01 result missing api_response network boundary"
            Assert-ArtifactValue $NetworkBoundary.kind "http_get" "Workflow 01 network boundary kind"
            Assert-ArtifactValue $NetworkBoundary.method "GET" "Workflow 01 network boundary method"
            Assert-ArtifactValue $NetworkBoundary.url "https://api.example.org/weather/hourly" "Workflow 01 network boundary URL"
            Assert-ArtifactValue $NetworkBoundary.response_source "offline_response" "Workflow 01 network response source"
            Assert-ArtifactValue $NetworkBoundary.status "offline_response" "Workflow 01 network boundary status"
            Assert-ArtifactValue $NetworkBoundary.status_code 200 "Workflow 01 network HTTP status code"
            Assert-ArtifactValue $NetworkBoundary.status_class "success" "Workflow 01 network HTTP status class"
            Assert-ArtifactValue $NetworkBoundary.retry 2 "Workflow 01 network retry count"
            Assert-ArtifactValue $NetworkBoundary.timeout "30 s" "Workflow 01 network timeout"
            Assert-ArtifactValue $NetworkBoundary.body_size_limit_bytes 2000000 "Workflow 01 network body limit"
            Assert-ArtifactValue $NetworkBoundary.expected_sha256 $WeatherResponseHash "Workflow 01 network expected hash"
            Assert-ArtifactValue $NetworkBoundary.response_hash $WeatherResponseHash "Workflow 01 network response hash"
            $NetworkQuery = @($NetworkBoundary.query)
            Assert-ArtifactNumber $NetworkQuery.Count 2 "Workflow 01 network query count"
            $StationQuery = @($NetworkQuery | Where-Object { [string]$_.key -eq "station" }) | Select-Object -First 1
            $YearQuery = @($NetworkQuery | Where-Object { [string]$_.key -eq "year" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $StationQuery) "Workflow 01 network query missing station"
            Assert-Artifact ($null -ne $YearQuery) "Workflow 01 network query missing year"
            Assert-ArtifactValue $StationQuery.value "STN001" "Workflow 01 network station query value"
            Assert-ArtifactValue $YearQuery.value "2024" "Workflow 01 network year query value"
            Assert-Artifact (-not [bool]$StationQuery.redacted) "Workflow 01 station query should not be redacted"
            Assert-Artifact (-not [bool]$YearQuery.redacted) "Workflow 01 year query should not be redacted"

            $ConfigPromotion = @($ResultData.typed_payload.config_promotions | Where-Object { [string]$_.binding -eq "api_contract" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $ConfigPromotion) "Workflow 01 result missing api_contract config promotion"
            Assert-ArtifactValue $ConfigPromotion.schema_name "WeatherApiPayload" "Workflow 01 config promotion schema"
            Assert-ArtifactValue $ConfigPromotion.source_value "api_response.body" "Workflow 01 config promotion source value"
            Assert-ArtifactValue $ConfigPromotion.status "validated" "Workflow 01 config promotion status"
            $WeatherHashRecord = @($ResultData.provenance.data_hashes | Where-Object { [string]$_.binding -eq "weather" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $WeatherHashRecord) "Workflow 01 result missing weather data hash"
            Assert-ArtifactValue $WeatherHashRecord.source_format "json_records" "Workflow 01 weather data source format"
            Assert-ArtifactValue $WeatherHashRecord.source_value "api_response.body" "Workflow 01 weather data source value"
            $CoverageRecord = @($ResultData.typed_payload.timeseries_coverage | Where-Object { [string]$_.binding -eq "coverage" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $CoverageRecord) "Workflow 01 result missing coverage record"
            Assert-ArtifactValue $CoverageRecord.source_table "weather" "Workflow 01 coverage source table"
            Assert-ArtifactValue $CoverageRecord.source_column "time" "Workflow 01 coverage source column"
            Assert-ArtifactValue $CoverageRecord.expected_count 8784 "Workflow 01 coverage expected leap-year count"
            Assert-ArtifactValue $CoverageRecord.actual_count 2 "Workflow 01 coverage actual fixture count"
            Assert-ArtifactValue $CoverageRecord.status "gapped" "Workflow 01 coverage status"

            $ReviewBoundary = @($ReviewData.review_document.external_boundaries | Where-Object { [string]$_.kind -eq "network_request" -and [string]$_.name -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $ReviewBoundary) "Workflow 01 review missing api_response network boundary"
            Assert-ArtifactValue $ReviewBoundary.method "GET" "Workflow 01 review network method"
            Assert-ArtifactValue $ReviewBoundary.target "https://api.example.org/weather/hourly" "Workflow 01 review network target"
            Assert-ArtifactValue $ReviewBoundary.response_source "offline_response" "Workflow 01 review network response source"
            Assert-ArtifactValue $ReviewBoundary.expected_sha256 $WeatherResponseHash "Workflow 01 review network expected hash"
            $ReviewCache = @($ReviewData.review_document.caches | Where-Object { [string]$_.owner_kind -eq "network_request" -and [string]$_.owner_name -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $ReviewCache) "Workflow 01 review missing api_response cache row"
            Assert-ArtifactValue $ReviewCache.expected_hash $WeatherResponseHash "Workflow 01 review cache expected hash"
            Assert-ArtifactValue $ReviewCache.observed_hash $WeatherResponseHash "Workflow 01 review cache observed hash"
            Assert-Artifact ($AllowedCacheStatuses -contains [string]$ReviewCache.status) "Workflow 01 review cache status should be $AllowedCacheStatusDescription, got $($ReviewCache.status)"
            $ReviewWeatherPromotion = @($ReviewData.csv_promotions | Where-Object { [string]$_.binding -eq "weather" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $ReviewWeatherPromotion) "Workflow 01 review missing weather JSON records promotion"
            Assert-ArtifactValue $ReviewWeatherPromotion.schema_name "WeatherApiRecord" "Workflow 01 review weather schema"
            Assert-ArtifactValue $ReviewWeatherPromotion.source_format "json_records" "Workflow 01 review weather source format"
            Assert-ArtifactValue $ReviewWeatherPromotion.source_value "api_response.body" "Workflow 01 review weather source value"

            $StandardWeatherArtifact = @($OutputManifestData.artifact_registry.generated_files | Where-Object { [string]$_.kind -eq "standard_file" -and [string]$_.path -eq "outputs/standard_weather_file.txt" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $StandardWeatherArtifact) "Workflow 01 output manifest missing standard weather artifact"
            Assert-ArtifactValue $StandardWeatherArtifact.status "generated" "Workflow 01 standard weather artifact status"
            $OutputNetworkRequest = @($OutputManifestData.artifact_registry.network_requests | Where-Object { [string]$_.binding -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $OutputNetworkRequest) "Workflow 01 output manifest missing api_response network request"
            Assert-ArtifactValue $OutputNetworkRequest.kind "http_get" "Workflow 01 output manifest network kind"
            Assert-ArtifactValue $OutputNetworkRequest.url "https://api.example.org/weather/hourly" "Workflow 01 output manifest network URL"
            Assert-ArtifactValue $OutputNetworkRequest.expected_sha256 $WeatherResponseHash "Workflow 01 output manifest expected hash"
            Assert-ArtifactValue $OutputNetworkRequest.response_hash $WeatherResponseHash "Workflow 01 output manifest response hash"
            Assert-ArtifactValue $OutputNetworkRequest.status "offline_response" "Workflow 01 output manifest network status"
            $OutputCache = @($OutputManifestData.artifact_registry.caches | Where-Object { [string]$_.binding -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $OutputCache) "Workflow 01 output manifest missing api_response cache"
            Assert-ArtifactValue $OutputCache.kind "network_request" "Workflow 01 output manifest cache kind"
            Assert-ArtifactValue $OutputCache.hash $WeatherResponseHash "Workflow 01 output manifest cache hash"
            Assert-Artifact ($AllowedCacheStatuses -contains [string]$OutputCache.status) "Workflow 01 output cache status should be $AllowedCacheStatusDescription, got $($OutputCache.status)"

            Assert-ArtifactNumber $CacheManifestData.cache_record_count 1 "Workflow 01 cache manifest record count"
            $CacheRecord = @($CacheManifestData.cache_records | Where-Object { [string]$_.owner_kind -eq "network_request" -and [string]$_.owner_name -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $CacheRecord) "Workflow 01 cache manifest missing api_response record"
            Assert-ArtifactValue $CacheRecord.expected_hash $WeatherResponseHash "Workflow 01 cache manifest expected hash"
            Assert-ArtifactValue $CacheRecord.observed_hash $WeatherResponseHash "Workflow 01 cache manifest observed hash"
            Assert-Artifact ($AllowedCacheStatuses -contains [string]$CacheRecord.status) "Workflow 01 cache manifest status should be $AllowedCacheStatusDescription, got $($CacheRecord.status)"
            Assert-Artifact ([string]$CacheRecord.cache_key -like "weather|demo|2024|source_hash=*") "Workflow 01 cache key should include weather/demo/2024/source_hash, got $($CacheRecord.cache_key)"

            Assert-ArtifactNumber $RunLogData.network_event_count 1 "Workflow 01 run log network event count"
            Assert-ArtifactNumber $RunLogData.cache_event_count 1 "Workflow 01 run log cache event count"
            $RunLogNetwork = @($RunLogData.network_events | Where-Object { [string]$_.binding -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $RunLogNetwork) "Workflow 01 run log missing api_response network event"
            Assert-ArtifactValue $RunLogNetwork.kind "http_get" "Workflow 01 run log network kind"
            Assert-ArtifactValue $RunLogNetwork.url "https://api.example.org/weather/hourly" "Workflow 01 run log network URL"
            Assert-ArtifactValue $RunLogNetwork.response_hash $WeatherResponseHash "Workflow 01 run log response hash"
            Assert-ArtifactValue $RunLogNetwork.status "offline_response" "Workflow 01 run log network status"
            $RunLogCache = @($RunLogData.cache_events | Where-Object { [string]$_.owner_kind -eq "network_request" -and [string]$_.owner_name -eq "api_response" }) | Select-Object -First 1
            Assert-Artifact ($null -ne $RunLogCache) "Workflow 01 run log missing api_response cache event"
            Assert-Artifact ($AllowedCacheStatuses -contains [string]$RunLogCache.status) "Workflow 01 run log cache status should be $AllowedCacheStatusDescription, got $($RunLogCache.status)"
        }
        if ($Workflow -like "*02_native_surrogate_case_workflow*") {
            $ResultPath = Join-Path $RepoRoot "build\result\result.engres"
            $ReviewPath = Join-Path $RepoRoot "build\result\review.json"
            $OutputManifestPath = Join-Path $RepoRoot "build\result\output_manifest.json"
            foreach ($RequiredWorkflowArtifactPath in @($ResultPath, $ReviewPath, $OutputManifestPath)) {
                if (-not (Test-Path -LiteralPath $RequiredWorkflowArtifactPath)) {
                    throw "Workflow 02 native contract smoke missing artifact: $RequiredWorkflowArtifactPath"
                }
            }
            $ResultJson = Get-Content -LiteralPath $ResultPath -Raw
            $ReviewJson = Get-Content -LiteralPath $ReviewPath -Raw
            $OutputManifestJson = Get-Content -LiteralPath $OutputManifestPath -Raw
            $ResultData = $ResultJson | ConvertFrom-Json
            $TypedPayload = $ResultData.typed_payload
            if ($null -eq $TypedPayload) {
                throw "Workflow 02 native result missing typed_payload"
            }
            $SampleTables = $ResultData.sample_tables
            if ($null -eq $SampleTables -and $null -ne $ResultData.typed_payload) {
                $SampleTables = $ResultData.typed_payload.sample_tables
            }
            foreach ($RequiredSampleTable in @(
                @{ Binding = "training_designs"; Seed = "42"; Count = 8 },
                @{ Binding = "designs"; Seed = "84"; Count = 3 }
            )) {
                $MatchingSampleTables = @($SampleTables | Where-Object {
                    [string]$_.binding -eq $RequiredSampleTable.Binding -and
                    [string]$_.generation -eq "sample_lhs" -and
                    [string]$_.method -eq "lhs" -and
                    [string]$_.seed -eq $RequiredSampleTable.Seed -and
                    [int]$_.sample_count -eq $RequiredSampleTable.Count -and
                    [int]$_.row_hash_count -eq $RequiredSampleTable.Count -and
                    @($_.duplicate_case_ids).Count -eq 0 -and
                    @($_.parameter_columns).Count -eq 6
                })
                if ($MatchingSampleTables.Count -ne 1) {
                    throw "Workflow 02 native sample table missing generated LHS contract for $($RequiredSampleTable.Binding)"
                }
                $SampleTable = $MatchingSampleTables[0]
                $RowHashPreview = @($SampleTable.row_hash_preview)
                if ($RowHashPreview.Count -eq 0 -or [string]$RowHashPreview[0] -notmatch "^[0-9a-f]{16,64}$") {
                    throw "Workflow 02 native sample table $($RequiredSampleTable.Binding) must expose generated row hash previews"
                }
                $RowPreview = @($SampleTable.row_preview)
                if ($RowPreview.Count -eq 0) {
                    throw "Workflow 02 native sample table $($RequiredSampleTable.Binding) must expose generated row value previews"
                }
                $FirstPreviewRow = $RowPreview[0]
                if ([string]$FirstPreviewRow.case_id -ne "case_001" -or [int]$FirstPreviewRow.row_number -ne 1) {
                    throw "Workflow 02 native sample table $($RequiredSampleTable.Binding) row preview must start with case_001 row 1"
                }
                $PreviewValues = @($FirstPreviewRow.values)
                if ($PreviewValues.Count -ne 6) {
                    throw "Workflow 02 native sample table $($RequiredSampleTable.Binding) row preview must include one numeric value per sampled parameter"
                }
                $PreviewValuesWithPayload = @($PreviewValues | Where-Object {
                    -not [string]::IsNullOrWhiteSpace([string]$_.column) -and
                    $null -ne $_.numeric_value -and
                    -not [string]::IsNullOrWhiteSpace([string]$_.unit)
                })
                if ($PreviewValuesWithPayload.Count -ne 6) {
                    throw "Workflow 02 native sample table $($RequiredSampleTable.Binding) row preview values must include column, numeric_value, and unit payloads"
                }
            }
            $ModelCards = @($TypedPayload.model_cards)
            $MatchingModelCards = @($ModelCards | Where-Object {
                [string]$_.binding -eq "surrogate_model" -and
                [string]$_.source -eq "training_results" -and
                [string]$_.model_kind -eq "linear" -and
                [string]$_.target -eq "annual_electricity" -and
                [string]$_.target_quantity -eq "Energy" -and
                [string]$_.target_unit -eq "kWh" -and
                [int]$_.train_count -eq 6 -and
                [int]$_.test_count -eq 2 -and
                [string]$_.status -eq "trained_linear" -and
                @($_.features).Count -eq 6
            })
            if ($MatchingModelCards.Count -ne 1) {
                throw "Workflow 02 native result missing structured regression model card contract"
            }
            $PredictionManifests = @($TypedPayload.prediction_manifests)
            $MatchingPredictionManifests = @($PredictionManifests | Where-Object {
                [string]$_.binding -eq "predictions" -and
                [string]$_.manifest_path -eq "native:predictions" -and
                [string]$_.model -eq "surrogate_model" -and
                $null -eq $_.model_file -and
                $null -eq $_.sample_file -and
                $null -eq $_.output_file -and
                [int]$_.row_count -eq 3 -and
                [string]$_.confidence_column -eq "confidence" -and
                [string]$_.status -eq "predicted" -and
                @($_.case_ids).Count -eq 3
            })
            if ($MatchingPredictionManifests.Count -ne 1) {
                throw "Workflow 02 native result missing native prediction manifest contract"
            }
            $DbManifests = @($TypedPayload.db_manifests)
            foreach ($RequiredDbManifest in @(
                @{ Binding = "training_results"; Table = "simulation_results"; RowCount = 8; Schema = @("case_id", "annual_electricity", "peak_cooling", "unmet_hours") },
                @{ Binding = "predictions"; Table = "predictions"; RowCount = 3; Schema = @("case_id", "predicted_annual_electricity", "confidence") }
            )) {
                $MatchingDbManifests = @($DbManifests | Where-Object {
                    [string]$_.binding -eq $RequiredDbManifest.Binding -and
                    [string]$_.database -eq "outputs/surrogate_results.sqlite" -and
                    [string]$_.transaction_status -eq "committed" -and
                    [string]$_.schema_status -eq "ok" -and
                    [string]$_.status -eq "manifest_loaded"
                })
                if ($MatchingDbManifests.Count -ne 1) {
                    throw "Workflow 02 native result missing DB manifest contract for $($RequiredDbManifest.Binding)"
                }
                $MatchingDbTables = @($MatchingDbManifests[0].tables | Where-Object {
                    [string]$_.name -eq $RequiredDbManifest.Table -and
                    [string]$_.mode -eq "replace" -and
                    [int]$_.row_count -eq $RequiredDbManifest.RowCount
                })
                if ($MatchingDbTables.Count -ne 1) {
                    throw "Workflow 02 native DB manifest missing table contract for $($RequiredDbManifest.Table)"
                }
                $DbTableSchema = @($MatchingDbTables[0].schema)
                foreach ($RequiredDbColumn in $RequiredDbManifest.Schema) {
                    if ($DbTableSchema -notcontains $RequiredDbColumn) {
                        throw "Workflow 02 native DB manifest table $($RequiredDbManifest.Table) missing column $RequiredDbColumn"
                    }
                }
            }
            $StructuredReads = @($TypedPayload.structured_reads)
            $MatchingStructuredReads = @($StructuredReads | Where-Object {
                [string]$_.binding -eq "persisted_predictions" -and
                [string]$_.kind -eq "sqlite" -and
                [string]$_.parse_status -eq "parsed" -and
                [string]$_.root_type -eq "sqlite_table" -and
                [int]$_.field_count -eq 3 -and
                [int]$_.item_count -eq 3 -and
                $null -eq $_.error -and
                [int]$_.line -gt 0 -and
                [string]$_.path -like "*surrogate_results.sqlite" -and
                [string]$_.source_hash -match "^[0-9a-f]{64}$"
            })
            if ($MatchingStructuredReads.Count -ne 1) {
                throw "Workflow 02 native result missing typed SQLite readback structured_read contract"
            }
            $CaseManifests = @($TypedPayload.case_manifests)
            foreach ($RequiredCaseManifestGroup in @(
                @{ SampleTable = "training_designs"; Count = 8 },
                @{ SampleTable = "designs"; Count = 3 }
            )) {
                $MatchingCaseManifests = @($CaseManifests | Where-Object {
                    [string]$_.sample_table -eq $RequiredCaseManifestGroup.SampleTable -and
                    [string]$_.source -eq "sample lhs" -and
                    [string]$_.status -eq "pending"
                })
                if ($MatchingCaseManifests.Count -ne $RequiredCaseManifestGroup.Count) {
                    throw "Workflow 02 native result missing generated case manifests for $($RequiredCaseManifestGroup.SampleTable)"
                }
            }
            $AllowedCaseManifestTables = @("training_designs", "designs")
            $UnexpectedCaseManifests = @($CaseManifests | Where-Object {
                $AllowedCaseManifestTables -notcontains [string]$_.sample_table -or
                [string]$_.source -ne "sample lhs" -or
                [string]$_.status -ne "pending" -or
                @($_.process_bindings).Count -ne 0 -or
                @($_.process_statuses).Count -ne 0
            } | Select-Object -First 1)
            if ($UnexpectedCaseManifests.Count -gt 0) {
                throw "Workflow 02 native case manifests must come from sample lhs without process bindings"
            }
            foreach ($RequiredSurrogateResultToken in @(
                '"sample_tables"',
                '"method": "lhs"',
                '"seed": "42"',
                '"case_manifests"',
                '"binding": "case_inputs"',
                '"schema_name": "CaseOutput"',
                '"status": "rendered"',
                '"binding": "case_result_collection"',
                '"schema_name": "CaseResultCollection"',
                '"source": "collect results case_inputs"',
                '"model_cards"',
                '"prediction_manifests"',
                '"schema_name": "PredictionResult"',
                '"db_manifests"',
                '"structured_reads"',
                '"binding": "persisted_predictions"',
                '"root_type": "sqlite_table"',
                '"item_count": 3',
                '"transaction_status": "committed"'
            )) {
                if (-not $ResultJson.Contains($RequiredSurrogateResultToken)) {
                    throw "Workflow 02 native result missing token $RequiredSurrogateResultToken"
                }
            }
            foreach ($RequiredSurrogateReviewToken in @(
                '"model_cards"',
                '"prediction_manifests"',
                '"db_manifests"',
                "case_result_collection.rows",
                '"binding": "case_inputs:case_001"'
            )) {
                if (-not $ReviewJson.Contains($RequiredSurrogateReviewToken)) {
                    throw "Workflow 02 native review missing token $RequiredSurrogateReviewToken"
                }
            }
            foreach ($RequiredSurrogateOutputToken in @(
                '"kind": "case_input"',
                '"kind": "template_render_manifest"',
                '"kind": "sqlite_database"',
                '"kind": "db_write_manifest"',
                '"path": "outputs/sampling_summary.txt"',
                '"path": "model://surrogate_model"',
                '"path": "model://predictions"',
                '"kind": "PredictionResult"'
            )) {
                if (-not $OutputManifestJson.Contains($RequiredSurrogateOutputToken)) {
                    throw "Workflow 02 native output manifest missing token $RequiredSurrogateOutputToken"
                }
            }
        }
        if ($Workflow -like "*03_uncertain_sensor_report*") {
            $ResultPath = Join-Path $RepoRoot "build\result\result.engres"
            $ReviewPath = Join-Path $RepoRoot "build\result\review.json"
            $OutputManifestPath = Join-Path $RepoRoot "build\result\output_manifest.json"
            $ReportSpecPath = Join-Path $RepoRoot "build\result\report_spec.json"
            $PlotSpecPath = Join-Path $RepoRoot "build\result\plots\plot_spec.json"
            $PlotManifestPath = Join-Path $RepoRoot "build\result\plots\plot_manifest.json"
            foreach ($RequiredWorkflowArtifactPath in @($ResultPath, $ReviewPath, $OutputManifestPath, $ReportSpecPath, $PlotSpecPath, $PlotManifestPath)) {
                if (-not (Test-Path -LiteralPath $RequiredWorkflowArtifactPath)) {
                    throw "Workflow 03 native contract smoke missing artifact: $RequiredWorkflowArtifactPath"
                }
            }
            $ResultJson = Get-Content -LiteralPath $ResultPath -Raw
            $ReviewJson = Get-Content -LiteralPath $ReviewPath -Raw
            $OutputManifestJson = Get-Content -LiteralPath $OutputManifestPath -Raw
            $ReportSpecJson = Get-Content -LiteralPath $ReportSpecPath -Raw
            $PlotSpecJson = Get-Content -LiteralPath $PlotSpecPath -Raw
            $PlotManifestJson = Get-Content -LiteralPath $PlotManifestPath -Raw
            $ResultData = $ResultJson | ConvertFrom-Json
            $ReviewData = $ReviewJson | ConvertFrom-Json
            $OutputManifestData = $OutputManifestJson | ConvertFrom-Json
            $ReportSpecData = $ReportSpecJson | ConvertFrom-Json
            $PlotSpecData = $PlotSpecJson | ConvertFrom-Json
            $PlotManifestData = $PlotManifestJson | ConvertFrom-Json

            $SensorCoverage = @($ResultData.typed_payload.timeseries_coverage | Where-Object {
                [string]$_.binding -eq "coverage" -and
                [string]$_.source_table -eq "sensor" -and
                [string]$_.source_column -eq "time" -and
                [int]$_.expected_count -eq 4 -and
                [int]$_.actual_count -eq 4 -and
                [int]$_.missing_count -eq 0 -and
                [string]$_.status -eq "complete"
            })
            Assert-ArtifactNumber $SensorCoverage.Count 1 "Workflow 03 native coverage contract count"

            $SensorUncertaintyCalcs = @($ResultData.typed_payload.timeseries_uncertainty_calculations)
            Assert-ArtifactNumber $SensorUncertaintyCalcs.Count 4 "Workflow 03 runtime uncertainty calculation count"
            foreach ($RequiredCalc in @(
                @{ Operation = "statistic"; Statistic = "mean"; Binding = $null; Method = "independent_pointwise_sensor_std_mean"; Unit = "W"; Nominal = 5072.43; Stddev = 100.0 },
                @{ Operation = "statistic"; Statistic = "duration_above(5 kW)"; Binding = $null; Method = "independent_pointwise_sensor_std_duration_above_finite_difference"; Unit = "s"; Nominal = 299.48325358851724; Stddev = 143.29363610942985 },
                @{ Operation = "integration"; Statistic = $null; Binding = "E_sensor"; Method = "independent_pointwise_sensor_std_trapezoidal"; Unit = "J"; Nominal = 4543242.0; Stddev = 94868.32980505138 }
            )) {
                $RequiredStatistic = $RequiredCalc.Statistic
                $RequiredBinding = $RequiredCalc.Binding
                $MatchingCalcs = @($SensorUncertaintyCalcs | Where-Object {
                    $StatisticMatches = if ($null -eq $RequiredStatistic) { $null -eq $_.statistic } else { [string]$_.statistic -eq [string]$RequiredStatistic }
                    $BindingMatches = if ($null -eq $RequiredBinding) { $null -eq $_.binding } else { [string]$_.binding -eq [string]$RequiredBinding }
                    [string]$_.source -eq "Q_sensor" -and
                    [string]$_.operation -eq [string]$RequiredCalc.Operation -and
                    $StatisticMatches -and
                    $BindingMatches -and
                    [string]$_.method -eq [string]$RequiredCalc.Method -and
                    [string]$_.status -eq "propagated_sensor_std" -and
                    [string]$_.unit -eq [string]$RequiredCalc.Unit -and
                    [double]$_.sensor_std -eq 0.2 -and
                    [string]$_.sensor_std_unit -eq "kW"
                })
                Assert-ArtifactNumber $MatchingCalcs.Count 1 "Workflow 03 propagated uncertainty calculation $($RequiredCalc.Method)"
                Assert-ArtifactFloat $MatchingCalcs[0].nominal_value ([double]$RequiredCalc.Nominal) "Workflow 03 nominal value for $($RequiredCalc.Method)" 0.000001
                Assert-ArtifactFloat $MatchingCalcs[0].stddev ([double]$RequiredCalc.Stddev) "Workflow 03 stddev for $($RequiredCalc.Method)" 0.000001
            }
            $P95MetadataOnly = @($SensorUncertaintyCalcs | Where-Object {
                [string]$_.source -eq "Q_sensor" -and
                [string]$_.operation -eq "statistic" -and
                [string]$_.statistic -eq "p95" -and
                [string]$_.method -eq "pointwise_sensor_std_metadata_only" -and
                [string]$_.status -eq "metadata_only" -and
                $null -eq $_.stddev
            })
            Assert-ArtifactNumber $P95MetadataOnly.Count 1 "Workflow 03 p95 uncertainty should remain explicitly metadata-only"

            $ReviewUncertainty = @($ReviewData.timeseries_uncertainty)
            Assert-ArtifactNumber $ReviewUncertainty.Count 1 "Workflow 03 review uncertainty row count"
            Assert-ArtifactValue $ReviewUncertainty[0].binding "Q_sensor" "Workflow 03 review uncertainty binding"
            Assert-ArtifactValue $ReviewUncertainty[0].sensor_std "0.2 kW" "Workflow 03 review uncertainty sensor_std"
            Assert-ArtifactValue $ReviewUncertainty[0].method "pointwise_measured_std" "Workflow 03 review uncertainty method"
            Assert-ArtifactValue $ReviewUncertainty[0].status "accepted" "Workflow 03 review uncertainty status"
            $ReviewUncertaintyCalcs = @($ReviewData.timeseries_uncertainty_calculations | Where-Object {
                [string]$_.source -eq "Q_sensor" -and
                [string]$_.sensor_std -eq "0.2 kW" -and
                [string]$_.status -eq "metadata_only"
            })
            Assert-ArtifactNumber $ReviewUncertaintyCalcs.Count 3 "Workflow 03 review uncertainty metadata row count"

            $ReportStatistics = @($ReportSpecData.computed_statistics | Where-Object {
                [string]$_.source -eq "Q_sensor" -and
                [string]$_.quantity_kind -eq "HeatRate" -and
                [string]$_.axis -eq "Time" -and
                [string]$_.status -eq "computed"
            })
            Assert-ArtifactNumber $ReportStatistics.Count 1 "Workflow 03 report computed statistics count"
            $ReportStatisticValues = @($ReportStatistics[0].values)
            Assert-ArtifactNumber $ReportStatisticValues.Count 3 "Workflow 03 report computed statistic value count"
            foreach ($RequiredReportValue in @(
                @{ Name = "mean"; Unit = "W" },
                @{ Name = "p95"; Unit = "W" },
                @{ Name = "duration_above(5 kW)"; Unit = "s" }
            )) {
                $MatchingReportValues = @($ReportStatisticValues | Where-Object {
                    [string]$_.name -eq [string]$RequiredReportValue.Name -and
                    [string]$_.unit -eq [string]$RequiredReportValue.Unit -and
                    $null -ne $_.value
                })
                Assert-ArtifactNumber $MatchingReportValues.Count 1 "Workflow 03 report computed statistic $($RequiredReportValue.Name)"
            }
            $ReportIntegrations = @($ReportSpecData.computed_integrations | Where-Object {
                [string]$_.binding -eq "E_sensor" -and
                [string]$_.source -eq "Q_sensor" -and
                [string]$_.over_axis -eq "Time" -and
                [string]$_.method -eq "trapezoidal" -and
                [string]$_.status -eq "computed"
            })
            Assert-ArtifactNumber $ReportIntegrations.Count 1 "Workflow 03 report computed integration count"
            Assert-ArtifactFloat $ReportIntegrations[0].value 4543242.0 "Workflow 03 report computed integration value" 0.000001

            $GeneratedSensorOutputs = @($OutputManifestData.artifact_registry.generated_files | Where-Object {
                [string]$_.status -eq "generated" -and
                [string]$_.validation.status -eq "passed" -and
                ([string]$_.path -eq "outputs/sensor_summary.csv" -or [string]$_.path -eq "outputs/sensor_quality_summary.txt")
            })
            Assert-ArtifactNumber $GeneratedSensorOutputs.Count 2 "Workflow 03 generated sensor output validation count"

            $PlotSeries = @($PlotSpecData.series)
            Assert-ArtifactNumber $PlotSeries.Count 1 "Workflow 03 plot series count"
            Assert-ArtifactValue $PlotSeries[0].name "Q_sensor" "Workflow 03 plot series name"
            Assert-ArtifactValue $PlotSeries[0].display_unit "kW" "Workflow 03 plot series display unit"
            $PlotBand = $PlotSeries[0].confidence_band
            Assert-ArtifactValue $PlotBand.method "pointwise_measured_std" "Workflow 03 plot confidence-band method"
            Assert-ArtifactValue $PlotBand.source "sensor_std" "Workflow 03 plot confidence-band source"
            Assert-ArtifactFloat $PlotBand.level 0.95 "Workflow 03 plot confidence-band level"
            Assert-ArtifactNumber @($PlotSeries[0].points).Count 4 "Workflow 03 plot point count"
            Assert-ArtifactNumber @($PlotBand.lower).Count 4 "Workflow 03 lower confidence-band point count"
            Assert-ArtifactNumber @($PlotBand.upper).Count 4 "Workflow 03 upper confidence-band point count"
            Assert-ArtifactFloat $PlotBand.lower[0][1] 4.48188 "Workflow 03 first lower confidence-band value" 0.000001
            Assert-ArtifactFloat $PlotBand.upper[3][1] 5.80928 "Workflow 03 last upper confidence-band value" 0.000001

            $PlotManifestPlots = @($PlotManifestData.plots)
            Assert-ArtifactNumber $PlotManifestPlots.Count 1 "Workflow 03 plot manifest plot count"
            Assert-ArtifactValue $PlotManifestPlots[0].svg "timeseries.svg" "Workflow 03 plot manifest SVG"
            Assert-Artifact (@($PlotManifestPlots[0].series) -contains "Q_sensor") "Workflow 03 plot manifest must list Q_sensor"
            foreach ($RequiredSensorResultToken in @(
                '"timeseries_uncertainty_calculations"',
                '"sensor_std": 0.2',
                '"sensor_std_unit": "kW"',
                '"method": "independent_pointwise_sensor_std_mean"',
                '"method": "independent_pointwise_sensor_std_trapezoidal"',
                '"status": "propagated_sensor_std"',
                '"timeseries_coverage"',
                '"uncertainties"'
            )) {
                if (-not $ResultJson.Contains($RequiredSensorResultToken)) {
                    throw "Workflow 03 native result missing token $RequiredSensorResultToken"
                }
            }
            foreach ($RequiredSensorReviewToken in @(
                '"timeseries_uncertainty"',
                '"timeseries_uncertainty_calculations"',
                '"sensor_std": "0.2 kW"',
                '"source": "Q_sensor"',
                '"key": "confidence_band"',
                '"value": "sensor_std"',
                '"kind": "timeseries_coverage"'
            )) {
                if (-not $ReviewJson.Contains($RequiredSensorReviewToken)) {
                    throw "Workflow 03 native review missing token $RequiredSensorReviewToken"
                }
            }
            foreach ($RequiredSensorOutputToken in @(
                '"kind": "csv_export"',
                '"path": "outputs/sensor_summary.csv"',
                '"kind": "write_text"',
                '"path": "outputs/sensor_quality_summary.txt"',
                '"kind": "plot_spec"',
                '"path": "plots/plot_spec.json"',
                '"kind": "plot_svg"',
                '"path": "plots/timeseries.svg"',
                '"kind": "plot_manifest"',
                '"path": "plots/plot_manifest.json"'
            )) {
                if (-not $OutputManifestJson.Contains($RequiredSensorOutputToken)) {
                    throw "Workflow 03 native output manifest missing token $RequiredSensorOutputToken"
                }
            }
            foreach ($RequiredSensorReportToken in @(
                '"uncertainty"',
                '"plot_manifest"',
                '"path": "plots/plot_manifest.json"',
                '"source": "Q_sensor"',
                '"operations": ["load_timeseries:Q_sensor", "integrate_over:Time", "store:E_sensor"]'
            )) {
                if (-not $ReportSpecJson.Contains($RequiredSensorReportToken)) {
                    throw "Workflow 03 native report spec missing token $RequiredSensorReportToken"
                }
            }
            foreach ($RequiredSensorPlotSpecToken in @(
                '"format": "eng-plotspec-v1"',
                '"plot_type": "line"',
                '"name": "Q_sensor"',
                '"confidence_band"',
                '"method": "pointwise_measured_std"',
                '"source": "sensor_std"'
            )) {
                if (-not $PlotSpecJson.Contains($RequiredSensorPlotSpecToken)) {
                    throw "Workflow 03 native plot spec missing token $RequiredSensorPlotSpecToken"
                }
            }
            foreach ($RequiredSensorPlotManifestToken in @(
                '"format": "eng-plot-manifest-v1"',
                '"plot_spec_version": 1',
                '"svg": "timeseries.svg"',
                '"series": ["Q_sensor"]'
            )) {
                if (-not $PlotManifestJson.Contains($RequiredSensorPlotManifestToken)) {
                    throw "Workflow 03 native plot manifest missing token $RequiredSensorPlotManifestToken"
                }
            }
        }
    }
}

function Invoke-WorkflowNativeStatus {
    $WorkflowRoot = Join-Path $RepoRoot "examples\workflows"
    $RequiredWorkflowSourcePaths = @(
        (Join-Path $WorkflowRoot "01_weather_api_to_standard_file\main.eng"),
        (Join-Path $WorkflowRoot "02_native_surrogate_case_workflow\main.eng"),
        (Join-Path $WorkflowRoot "03_uncertain_sensor_report\main.eng")
    )
    foreach ($RequiredWorkflowSourcePath in $RequiredWorkflowSourcePaths) {
        if (-not (Test-Path -LiteralPath $RequiredWorkflowSourcePath -PathType Leaf)) {
            throw "Native workflow status missing required workflow entrypoint: $RequiredWorkflowSourcePath"
        }
    }

    $ForbiddenNativeWorkflowMarkers = @(
        "\bpython(?:\d+(?:\.\d+)*)?(?:\.exe)?\b",
        "\bpy(?:\.exe)?\b",
        "\.py\b",
        "\.pyw\b",
        "\.ipynb\b",
        "\bpip(?:3)?\b",
        "\bconda\b",
        "\bpoetry\b",
        "\bpyenv\b",
        "\bmamba\b",
        "\bmicromamba\b",
        "\bvirtualenv\b",
        "\bvenv\b",
        "\bipython\b",
        "\bpytest\b",
        "\btox\b",
        "\bnox\b",
        "\bmypy\b",
        "\bruff\b",
        "\bsubprocess\b",
        "\bpandas\b",
        "\bnumpy\b",
        "\bscipy\b",
        "\bsklearn\b",
        "\bstatsmodels\b",
        "\bpolars\b",
        "\bmatplotlib\b",
        "\brequests\b",
        "\burllib\b",
        "\bpyarrow\b",
        "\bxarray\b",
        "\btensorflow\b",
        "\bpytorch\b",
        "\btorch\b",
        "\bjupyter\b",
        "\bjupyterlab\b",
        "\bnotebook\b"
    )
    $ForbiddenNativeWorkflowDocMarkers = @($ForbiddenNativeWorkflowMarkers | Where-Object {
        $_ -ne "\brequests\b" -and $_ -ne "\burllib\b"
    })
    $NativeWorkflowSourceAuditPaths = @($RequiredWorkflowSourcePaths | ForEach-Object {
        Split-Path -Parent $_
    } | Sort-Object -Unique | ForEach-Object {
        Get-ChildItem -LiteralPath $_ -Recurse -File -Filter "*.eng"
    } | ForEach-Object {
        $_.FullName
    } | Sort-Object -Unique)
    $WorkflowPublicDocPaths = @(
        @(Get-ChildItem -LiteralPath $WorkflowRoot -Recurse -File -Include "*.md", "*.txt" | Sort-Object FullName)
        @(Get-ChildItem -LiteralPath (Join-Path $RepoRoot "docs\workflows") -File -Filter "*.md" | Sort-Object FullName)
        @(
            "examples\README.md",
            "docs\user\tutorial\12_composite_workflow.md",
            "docs\current\workflow_modules.md",
            "docs\current\test_ci_gates.md"
        ) | ForEach-Object {
            Get-Item -LiteralPath (Join-Path $RepoRoot $_)
        }
    ) | Sort-Object FullName -Unique
    $ForbiddenWorkflowDocWording = @(
        "files produced by an external process",
        "external-simulator adapter pattern",
        "native surrogate half",
        "external simulator adapter could feed later",
        "Python process:",
        "created by Python",
        "Python-created",
        "generated by Python",
        "Python-generated",
        "Python-made",
        "Python-backed",
        "Python-side",
        "CSV fixture",
        "02_external_simulation_surrogate",
        "external_simulation_surrogate.md"
    )

    $Issues = New-Object System.Collections.Generic.List[string]
    foreach ($NativeWorkflowSourceAuditPath in $NativeWorkflowSourceAuditPaths) {
        $Workflow = $NativeWorkflowSourceAuditPath.Substring($RepoRoot.Length).TrimStart('\')
        $WorkflowSource = Get-Content -LiteralPath $NativeWorkflowSourceAuditPath -Raw
        if ($WorkflowSource -match "(?im)\brun\s+command\b") {
            $Issues.Add("source uses run command: $Workflow") | Out-Null
        }
        foreach ($PythonMarker in $ForbiddenNativeWorkflowMarkers) {
            if ($WorkflowSource -match "(?i)$PythonMarker") {
                $Issues.Add("source contains Python/notebook marker $PythonMarker`: $Workflow") | Out-Null
            }
        }
        if ($WorkflowSource -match "(?i)\bselect_first_row\s*\(") {
            $Issues.Add("source uses legacy select_first_row instead of filter + require_one: $Workflow") | Out-Null
        }
    }
    foreach ($WorkflowPublicDocPath in $WorkflowPublicDocPaths) {
        $WorkflowPublicDoc = Get-Content -LiteralPath $WorkflowPublicDocPath.FullName -Raw
        if ($WorkflowPublicDoc -match "(?im)\brun\s+command\b") {
            $Issues.Add("public docs describe workflow 01/02/03 as run-command backed: $($WorkflowPublicDocPath.FullName)") | Out-Null
        }
        foreach ($PythonMarker in $ForbiddenNativeWorkflowDocMarkers) {
            if ($WorkflowPublicDoc -match "(?i)$PythonMarker") {
                $Issues.Add("public docs contain Python/notebook marker $PythonMarker`: $($WorkflowPublicDocPath.FullName)") | Out-Null
            }
        }
        foreach ($ForbiddenWorkflowDocPhrase in $ForbiddenWorkflowDocWording) {
            if ($WorkflowPublicDoc.Contains($ForbiddenWorkflowDocPhrase)) {
                $Issues.Add("public docs contain stale external-process wording '$ForbiddenWorkflowDocPhrase': $($WorkflowPublicDocPath.FullName)") | Out-Null
            }
        }
    }

    $ProcessArtifactStatus = "missing; run .\dev.bat workflows-test for fresh artifact evidence"
    $ProcessResultsPath = Join-Path $RepoRoot "build\result\process_results.json"
    if (Test-Path -LiteralPath $ProcessResultsPath -PathType Leaf) {
        try {
            $ProcessResults = Get-Content -LiteralPath $ProcessResultsPath -Raw | ConvertFrom-Json
            $ProcessCount = 0
            if ($null -ne $ProcessResults.process_count) {
                $ProcessCount = [int]$ProcessResults.process_count
            }
            $ProcessListCount = 0
            if ($null -ne $ProcessResults.processes) {
                $ProcessListCount = @($ProcessResults.processes).Count
            }
            $ProcessArtifactStatus = "present; format=$($ProcessResults.format); profile=$($ProcessResults.execution_profile); process_count=$ProcessCount; processes=$ProcessListCount"
            if ([string]$ProcessResults.format -ne "eng-process-results-v1") {
                $Issues.Add("latest process_results.json has unexpected format $($ProcessResults.format)") | Out-Null
            }
            if ([string]$ProcessResults.execution_profile -ne "normal") {
                $Issues.Add("latest process_results.json has unexpected execution_profile $($ProcessResults.execution_profile)") | Out-Null
            }
            if ($ProcessCount -ne 0 -or $ProcessListCount -ne 0) {
                $Issues.Add("latest process_results.json records external processes") | Out-Null
            }
        } catch {
            $Issues.Add("could not parse latest process_results.json: $($_.Exception.Message)") | Out-Null
        }
    }

    $RunGraphStatus = "missing; run .\dev.bat workflows-test for fresh graph evidence"
    $RunGraphPaths = @(
        (Join-Path $RepoRoot "build\result\static_run_plan.json"),
        (Join-Path $RepoRoot "build\result\run_plan.json")
    )
    $ExistingRunGraphPaths = @($RunGraphPaths | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
    if ($ExistingRunGraphPaths.Count -gt 0) {
        $RunGraphStatus = "present; checked $($ExistingRunGraphPaths.Count) graph artifact(s)"
    }
    foreach ($NativeWorkflowRunGraphPath in $ExistingRunGraphPaths) {
        try {
            $NativeWorkflowRunGraph = Get-Content -LiteralPath $NativeWorkflowRunGraphPath -Raw | ConvertFrom-Json
            foreach ($NativeWorkflowRunGraphNode in @($NativeWorkflowRunGraph.graph.nodes)) {
                foreach ($NativeWorkflowRunGraphField in @(
                    [string]$NativeWorkflowRunGraphNode.id,
                    [string]$NativeWorkflowRunGraphNode.kind,
                    [string]$NativeWorkflowRunGraphNode.label
                )) {
                    if ($NativeWorkflowRunGraphField -match "(?i)^process:" -or $NativeWorkflowRunGraphField -match "(?i)\brun\s+command\b") {
                        $Issues.Add("run graph contains process/run-command node metadata '$NativeWorkflowRunGraphField': $NativeWorkflowRunGraphPath") | Out-Null
                    }
                    foreach ($PythonMarker in $ForbiddenNativeWorkflowMarkers) {
                        if ($NativeWorkflowRunGraphField -match "(?i)$PythonMarker") {
                            $Issues.Add("run graph contains Python/notebook marker $PythonMarker in '$NativeWorkflowRunGraphField': $NativeWorkflowRunGraphPath") | Out-Null
                        }
                    }
                }
            }
            foreach ($NativeWorkflowRunGraphEdge in @($NativeWorkflowRunGraph.graph.edges)) {
                foreach ($NativeWorkflowRunGraphField in @(
                    [string]$NativeWorkflowRunGraphEdge.from,
                    [string]$NativeWorkflowRunGraphEdge.to,
                    [string]$NativeWorkflowRunGraphEdge.kind
                )) {
                    if ($NativeWorkflowRunGraphField -match "(?i)^process:" -or $NativeWorkflowRunGraphField -match "(?i)\brun\s+command\b") {
                        $Issues.Add("run graph contains process/run-command edge metadata '$NativeWorkflowRunGraphField': $NativeWorkflowRunGraphPath") | Out-Null
                    }
                    foreach ($PythonMarker in $ForbiddenNativeWorkflowMarkers) {
                        if ($NativeWorkflowRunGraphField -match "(?i)$PythonMarker") {
                            $Issues.Add("run graph contains Python/notebook marker $PythonMarker in '$NativeWorkflowRunGraphField': $NativeWorkflowRunGraphPath") | Out-Null
                        }
                    }
                }
            }
        } catch {
            $Issues.Add("could not parse run graph $NativeWorkflowRunGraphPath`: $($_.Exception.Message)") | Out-Null
        }
    }

    if ($Issues.Count -gt 0) {
        throw "Native workflow status failed:`n - $($Issues -join "`n - ")"
    }

    Write-Host "Native workflow status"
    Write-Host "  source guard: passed ($(@($NativeWorkflowSourceAuditPaths).Count) source file(s))"
    Write-Host "  public docs guard: passed ($(@($WorkflowPublicDocPaths).Count) doc file(s))"
    Write-Host "  latest process artifact: $ProcessArtifactStatus"
    Write-Host "  latest run graph artifact: $RunGraphStatus"
    Write-Host "  full evidence gate: .\dev.bat workflows-test"
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
        [string] $WorkflowDocsPath,

        [Parameter(Mandatory = $true)]
        [string] $CliSpecPath
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
    if (-not (Test-Path -LiteralPath $CliSpecPath -PathType Leaf)) {
        throw "missing CLI spec docs at $CliSpecPath"
    }

    $RegistryText = Get-Content -LiteralPath $RegistryPath -Raw -Encoding UTF8
    $ReadmeText = Get-Content -LiteralPath $ReadmePath -Raw -Encoding UTF8
    $WorkflowDocsText = Get-Content -LiteralPath $WorkflowDocsPath -Raw -Encoding UTF8
    $CliSpecText = Get-Content -LiteralPath $CliSpecPath -Raw -Encoding UTF8
    $CliDiagnosticCodes = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($match in [regex]::Matches($CliSpecText, '\b[EW]-[A-Z0-9][A-Z0-9_-]*\b')) {
        [void]$CliDiagnosticCodes.Add($match.Value)
    }
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
    $MissingCliDiagnostics = New-Object System.Collections.Generic.List[string]
    foreach ($entry in $RegistryEntries) {
        foreach ($field in @("status", "backing", "purpose", "artifacts", "diagnostics", "examples", "tests")) {
            if (-not $entry.Contains($field)) {
                throw "module registry entry $($entry.name) is missing $field"
            }
        }
        foreach ($diagnostic in @($entry.diagnostics)) {
            if ($diagnostic -notmatch '^[EW]-[A-Z0-9][A-Z0-9_-]*$') {
                throw "module registry entry $($entry.name) has non-diagnostic diagnostics value '$diagnostic'"
            }
            if (-not $CliDiagnosticCodes.Contains($diagnostic)) {
                $MissingCliDiagnostics.Add("$diagnostic ($($entry.name))") | Out-Null
            }
        }
    }
    if ($MissingCliDiagnostics.Count -gt 0) {
        Write-Host "Module registry diagnostics missing from docs/reference/cli/spec.md:"
        $MissingCliDiagnostics | Sort-Object | ForEach-Object { Write-Host "  $_" }
        throw "Module registry CLI diagnostic coverage check failed."
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

function Test-StdlibReferenceDocs {
    param(
        [Parameter(Mandatory = $true)]
        [string] $ReferencePath
    )

    if (-not (Test-Path -LiteralPath $ReferencePath -PathType Leaf)) {
        throw "missing stdlib reference at $ReferencePath"
    }

    $text = Get-Content -LiteralPath $ReferencePath -Raw -Encoding UTF8
    foreach ($stalePhrase in @("Status: early index", "Planned modules:")) {
        if ($text.Contains($stalePhrase)) {
            throw "stdlib reference still contains stale phrase: $stalePhrase"
        }
    }

    foreach ($requiredPhrase in @(
        "stdlib/eng/modules.toml",
        "docs-check",
        "Native workflow support",
        "eng.net",
        "eng.cache",
        "eng.sampling",
        "eng.case",
        "eng.db",
        "eng.model"
    )) {
        if (-not $text.Contains($requiredPhrase)) {
            throw "stdlib reference missing required current-scope phrase: $requiredPhrase"
        }
    }

    Write-Host "Stdlib reference docs wording check passed."
}

function Test-ExamplesReferenceDocs {
    param(
        [Parameter(Mandatory = $true)]
        [string] $ExamplesReadmePath
    )

    if (-not (Test-Path -LiteralPath $ExamplesReadmePath -PathType Leaf)) {
        throw "missing examples README at $ExamplesReadmePath"
    }

    $text = Get-Content -LiteralPath $ExamplesReadmePath -Raw -Encoding UTF8
    foreach ($stalePhrase in @("External process seed", "implementation seed")) {
        if ($text.Contains($stalePhrase)) {
            throw "examples README still contains stale phrase: $stalePhrase"
        }
    }
    foreach ($requiredPhrase in @("External process surface", "ProcessResult", "process_results.json")) {
        if (-not $text.Contains($requiredPhrase)) {
            throw "examples README missing required process example phrase: $requiredPhrase"
        }
    }

    Write-Host "Examples README wording check passed."
}

function Test-PublicWorkflowDocs {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Paths
    )

    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "missing public workflow doc at $path"
        }
        $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8
        foreach ($stalePhrase in @(
            "workflow skeletons",
            "External process seed",
            "implementation seed",
            "uncertainty and data-driven modeling seeds",
            "component-solver seeds",
            "case_inputs.planned_count",
            "case-input planned/blocked counts",
            "remaining planned counts",
            "seeded Monte Carlo"
        )) {
            if ($text.Contains($stalePhrase)) {
                throw "public workflow doc still contains stale phrase '$stalePhrase' at $path"
            }
        }
    }

    Write-Host "Public workflow docs wording check passed."
}


function Test-ContainsByteSequence {
    param(
        [Parameter(Mandatory = $true)]
        [byte[]] $Bytes,
        [Parameter(Mandatory = $true)]
        [byte[]] $Pattern
    )

    if ($Pattern.Count -eq 0 -or $Bytes.Count -lt $Pattern.Count) {
        return $false
    }
    for ($index = 0; $index -le $Bytes.Count - $Pattern.Count; $index++) {
        $matched = $true
        for ($offset = 0; $offset -lt $Pattern.Count; $offset++) {
            if ($Bytes[$index + $offset] -ne $Pattern[$offset]) {
                $matched = $false
                break
            }
        }
        if ($matched) {
            return $true
        }
    }
    return $false
}

function Test-NoCelsiusMojibake {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Paths
    )

    $badTokens = @(
        "$([char]0xC9F8)C",
        "$([char]0xF9DE)$([char]0xD1D2)",
        "$([char]0x7B4C)$([char]0xC68F)$([char]0xB1ED)"
    )
    $badBytePatterns = @(
        [byte[]](0xA1, 0xC6, 0x43),
        [byte[]](0xEC, 0xA7, 0xB8, 0x43),
        [byte[]](0xEF, 0xA7, 0x9E, 0xED, 0x87, 0x92),
        [byte[]](0xE7, 0xAD, 0x8C, 0xEC, 0x9A, 0x8F, 0xEB, 0x87, 0xAD)
    )
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
        }
        $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8
        foreach ($badToken in $badTokens) {
            if ($text.Contains($badToken)) {
                throw "Celsius alias text is mojibake at $path; use degC or `$([char]0x00B0)C instead."
            }
        }
        $bytes = [System.IO.File]::ReadAllBytes($path)
        foreach ($badPattern in $badBytePatterns) {
            if (Test-ContainsByteSequence -Bytes $bytes -Pattern $badPattern) {
                throw "Celsius alias bytes are mojibake at $path; use UTF-8 degC or `$([char]0x00B0)C instead."
            }
        }
    }
}

function Test-CurrentDocsImplementationWording {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Paths
    )

    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "missing current documentation wording input at $path"
        }
        $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8
        foreach ($stalePhrase in @(
            "Implementation seeds",
            "implementation seed",
            "implementation seeds",
            "metadata-only solver_plan seeds",
            "native VM seed",
            "Report seed",
            "seeded Monte Carlo workflows",
            "claim stable Monte Carlo semantics before seeded reproducibility is enforced"
        )) {
            if ($text.IndexOf($stalePhrase, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
                throw "current docs still contain stale implementation wording '$stalePhrase' at $path"
            }
        }
    }

    Write-Host "Current docs implementation wording check passed."
}

function Test-ArchitectureDocsWording {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Paths
    )

    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "missing architecture doc at $path"
        }
        $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8
        foreach ($stalePhrase in @(
            "frontend skeleton",
            "semantic skeleton",
            "TypedBinding skeleton",
            "expected type internal API skeleton",
            "quantity completion data table skeleton",
            "quantity completion skeleton",
            "future IDE completion source"
        )) {
            if ($text.Contains($stalePhrase)) {
                throw "architecture doc still contains stale implementation wording '$stalePhrase' at $path"
            }
        }
    }

    Write-Host "Architecture docs wording check passed."
}

function Test-UserDocsExecutionWording {
    param(
        [Parameter(Mandatory = $true)]
        [string] $UserDocsRoot
    )

    if (-not (Test-Path -LiteralPath $UserDocsRoot -PathType Container)) {
        throw "missing user docs root at $UserDocsRoot"
    }

    foreach ($docPath in Get-ChildItem -LiteralPath $UserDocsRoot -Recurse -Filter "*.md") {
        $text = Get-Content -LiteralPath $docPath.FullName -Raw -Encoding UTF8
        if ($text -match "(?m)^##\s+Run Commands?\s*$") {
            throw "user docs should use execution wording instead of Run Command heading: $($docPath.FullName)"
        }
        if ($text -match "(?i)\brun commands\b") {
            throw "user docs should say execute commands instead of run commands: $($docPath.FullName)"
        }
        if ($text.Contains("seeded Monte Carlo")) {
            throw "user docs should describe Monte Carlo scope without seed-centered workflow wording: $($docPath.FullName)"
        }
    }

    Write-Host "User docs execution wording check passed."
}

function Test-StdlibModuleBoundaryNotes {
    param(
        [Parameter(Mandatory = $true)]
        [string] $StdlibRoot
    )

    if (-not (Test-Path -LiteralPath $StdlibRoot -PathType Container)) {
        throw "missing stdlib module root at $StdlibRoot"
    }

    $stalePhrases = @(
        "planned native apply syntax",
        "Native apply/run/collect syntax and automatic case directory execution remain",
        "Workflow examples still select and render cases explicitly",
        "structured json/toml promotion remains a future eng.config boundary",
        "#   mkdir path"
    )
    foreach ($moduleFile in Get-ChildItem -LiteralPath $StdlibRoot -Filter "*.eng") {
        $moduleText = Get-Content -LiteralPath $moduleFile.FullName -Raw -Encoding UTF8
        foreach ($stalePhrase in $stalePhrases) {
            if ($moduleText.Contains($stalePhrase)) {
                throw "stdlib module note $($moduleFile.Name) still contains stale phrase: $stalePhrase"
            }
        }
    }

    Write-Host "Stdlib module boundary notes wording check passed."
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
    $CelsiusMojibakeCheckFiles = @($markdownFiles.ToArray()) + @(
        (Join-Path $RepoRoot "crates\eng_compiler\src\lib.rs"),
        (Join-Path $RepoRoot "crates\eng_compiler\src\units.rs"),
        (Join-Path $RepoRoot "stdlib\units.eng")
    )
    Test-NoCelsiusMojibake -Paths $CelsiusMojibakeCheckFiles
    Test-MarkdownLinks -Files $linkFiles.ToArray()
    Test-ModuleRegistryDocs `
        -RegistryPath (Join-Path $RepoRoot "stdlib\eng\modules.toml") `
        -ReadmePath (Join-Path $RepoRoot "stdlib\README.md") `
        -WorkflowDocsPath (Join-Path $RepoRoot "docs\current\workflow_modules.md") `
        -CliSpecPath (Join-Path $RepoRoot "docs\reference\cli\spec.md")
    Test-StdlibReferenceDocs `
        -ReferencePath (Join-Path $RepoRoot "docs\reference\stdlib\index.md")
    Test-StdlibModuleBoundaryNotes `
        -StdlibRoot (Join-Path $RepoRoot "stdlib\eng")
    Test-ExamplesReferenceDocs `
        -ExamplesReadmePath (Join-Path $RepoRoot "examples\README.md")
    Test-PublicWorkflowDocs -Paths @(
        (Join-Path $RepoRoot "docs\current\feature_maturity_matrix.md"),
        (Join-Path $RepoRoot "docs\current\tracks.md"),
        (Join-Path $RepoRoot "docs\current\workflow_modules.md"),
        (Join-Path $RepoRoot "docs\workflows\index.md"),
        (Join-Path $RepoRoot "docs\workflows\native_surrogate_case_workflow.md"),
        (Join-Path $RepoRoot "docs\release\v0.1.0.md"),
        (Join-Path $RepoRoot "examples\workflows\02_native_surrogate_case_workflow\expected\review_summary.md")
    )
    Test-CurrentDocsImplementationWording -Paths @(
        (Join-Path $RepoRoot "LLM_CONTEXT.md"),
        (Join-Path $RepoRoot "docs\development\05_historical_stable_core_gap_audit.md"),
        (Join-Path $RepoRoot "docs\internal\runtime\bytecode.md"),
        (Join-Path $RepoRoot "docs\internal\solver\README.md")
    )
    Test-ArchitectureDocsWording -Paths @(
        (Join-Path $RepoRoot "docs\architecture\02_compiler_frontend.md"),
        (Join-Path $RepoRoot "docs\architecture\03_expected_types_and_quantities.md")
    )
    Test-UserDocsExecutionWording `
        -UserDocsRoot (Join-Path $RepoRoot "docs\user")

    $ReportReferencePath = Join-Path $RepoRoot "docs\reference\language\report.md"
    $ReportReferenceSource = Get-Content -LiteralPath $ReportReferencePath -Raw
    foreach ($ForbiddenReportReferenceExample in @("show summary", "plot heat over Time")) {
        if ($ReportReferenceSource.Contains($ForbiddenReportReferenceExample)) {
            throw "Report language reference must use concrete supported bindings instead of '$ForbiddenReportReferenceExample'"
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
    Assert-ArtifactNumber @($reviewSystemIr.solver_plan.jacobian_sparsity).Count $Golden.review.jacobian_sparsity_count "system review solver_plan.jacobian_sparsity count"
    Assert-ArtifactNumber @($reviewSystemIr.solver_plan.jacobian_seed).Count $Golden.review.jacobian_seed_count "system review solver_plan.jacobian_seed compatibility count"
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
    Assert-ArtifactNumber @($reportSystemIr.solver_plan.jacobian_sparsity).Count $Golden.report_spec.jacobian_sparsity_count "system report_spec solver_plan.jacobian_sparsity count"
    Assert-ArtifactNumber @($reportSystemIr.solver_plan.jacobian_seed).Count $Golden.report_spec.jacobian_seed_count "system report_spec solver_plan.jacobian_seed compatibility count"
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
    Assert-ArtifactNumber @($resultSystemIr.solver_plan.jacobian_sparsity).Count $Golden.result.jacobian_sparsity_count "system result solver_plan.jacobian_sparsity count"
    Assert-ArtifactNumber @($resultSystemIr.solver_plan.jacobian_seed).Count $Golden.result.jacobian_seed_count "system result solver_plan.jacobian_seed compatibility count"
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

    $pattern = "(?:pub\s+)?const\s+$([regex]::Escape($Name))\s*:\s*&\[\&str\]\s*=\s*&\[(?<body>.*?)\];"
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

function Assert-JavaScriptStructuralBalance {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    $Source = Get-Content -LiteralPath $Path -Raw
    $Stack = New-Object System.Collections.Generic.List[object]
    $Line = 1
    $Column = 0
    $Index = 0
    $State = "code"
    $PreviousSignificant = ""
    $RegexInClass = $false
    $StringStartLine = 1
    $StringStartColumn = 1
    $SingleQuote = [char]39
    $DoubleQuote = [char]34
    $Backtick = [char]96
    $Backslash = [char]92
    $Slash = [char]47
    $Asterisk = [char]42
    $Cr = [char]13
    $Lf = [char]10
    $Openers = @{ "{" = "}"; "[" = "]"; "(" = ")" }
    $Closers = @{ "}" = "{"; "]" = "["; ")" = "(" }

    while ($Index -lt $Source.Length) {
        $Ch = $Source[$Index]
        $Next = if ($Index + 1 -lt $Source.Length) { $Source[$Index + 1] } else { [char]0 }

        if ($State -eq "lineComment") {
            if ($Ch -eq $Cr -or $Ch -eq $Lf) {
                if ($Ch -eq $Cr -and $Next -eq $Lf) { $Index += 1 }
                $Line += 1
                $Column = 0
                $State = "code"
            } else {
                $Column += 1
            }
            $Index += 1
            continue
        }

        if ($State -eq "blockComment") {
            if ($Ch -eq $Cr -or $Ch -eq $Lf) {
                if ($Ch -eq $Cr -and $Next -eq $Lf) { $Index += 1 }
                $Line += 1
                $Column = 0
            } elseif ($Ch -eq $Asterisk -and $Next -eq $Slash) {
                $Index += 1
                $Column += 2
                $State = "code"
            } else {
                $Column += 1
            }
            $Index += 1
            continue
        }

        if ($State -eq "regexLiteral") {
            if ($Ch -eq $Cr -or $Ch -eq $Lf) {
                throw "$Path has unterminated JavaScript regular expression literal starting at line $StringStartLine, column $StringStartColumn"
            }
            if ($Ch -eq $Backslash) {
                $Index += 2
                $Column += 2
                continue
            }
            if ($Ch -eq [char]91) {
                $RegexInClass = $true
            } elseif ($Ch -eq [char]93) {
                $RegexInClass = $false
            } elseif ($Ch -eq $Slash -and -not $RegexInClass) {
                $State = "code"
                $PreviousSignificant = "regex"
                $Column += 1
                $Index += 1
                continue
            }
            $Column += 1
            $Index += 1
            continue
        }

        if ($State -eq "singleString" -or $State -eq "doubleString" -or $State -eq "templateString") {
            $Terminator = if ($State -eq "singleString") { $SingleQuote } elseif ($State -eq "doubleString") { $DoubleQuote } else { $Backtick }
            if (($State -eq "singleString" -or $State -eq "doubleString") -and ($Ch -eq $Cr -or $Ch -eq $Lf)) {
                throw "$Path has unterminated JavaScript string starting at line $StringStartLine, column $StringStartColumn"
            }
            if ($Ch -eq $Backslash) {
                $Index += 2
                $Column += 2
                continue
            }
            if ($Ch -eq $Terminator) {
                $State = "code"
                $PreviousSignificant = "string"
                $Column += 1
                $Index += 1
                continue
            }
            if ($Ch -eq $Cr -or $Ch -eq $Lf) {
                if ($Ch -eq $Cr -and $Next -eq $Lf) { $Index += 1 }
                $Line += 1
                $Column = 0
            } else {
                $Column += 1
            }
            $Index += 1
            continue
        }

        if ($Ch -eq $Cr -or $Ch -eq $Lf) {
            if ($Ch -eq $Cr -and $Next -eq $Lf) { $Index += 1 }
            $Line += 1
            $Column = 0
            $Index += 1
            continue
        }

        if ($Ch -eq $Slash -and $Next -ne $Slash -and $Next -ne $Asterisk) {
            $PrefixStart = [Math]::Max(0, $Index - 16)
            $PrefixLength = $Index - $PrefixStart
            $Prefix = if ($PrefixLength -gt 0) { $Source.Substring($PrefixStart, $PrefixLength) } else { "" }
            $RegexPrefixTokens = @("", "(", "[", "{", "=", ":", ",", ";", "!", "?", "+", "-", "*", "%", "&", "|")
            if (($RegexPrefixTokens -contains $PreviousSignificant) -or $Prefix -match '(^|[^A-Za-z0-9_$])(return|throw|case|delete|typeof|void|new|in|of)\s*$') {
                $State = "regexLiteral"
                $RegexInClass = $false
                $StringStartLine = $Line
                $StringStartColumn = $Column + 1
                $Index += 1
                $Column += 1
                continue
            }
        }

        if ($Ch -eq $Slash -and $Next -eq $Slash) {
            $State = "lineComment"
            $Index += 2
            $Column += 2
            continue
        }
        if ($Ch -eq $Slash -and $Next -eq $Asterisk) {
            $State = "blockComment"
            $Index += 2
            $Column += 2
            continue
        }
        if ($Ch -eq $SingleQuote -or $Ch -eq $DoubleQuote -or $Ch -eq $Backtick) {
            $State = if ($Ch -eq $SingleQuote) { "singleString" } elseif ($Ch -eq $DoubleQuote) { "doubleString" } else { "templateString" }
            $StringStartLine = $Line
            $StringStartColumn = $Column + 1
            $Index += 1
            $Column += 1
            continue
        }

        $Token = [string]$Ch
        if ($Openers.ContainsKey($Token)) {
            $Stack.Add([pscustomobject]@{ Token = $Token; Line = $Line; Column = $Column + 1 }) | Out-Null
        } elseif ($Closers.ContainsKey($Token)) {
            if ($Stack.Count -eq 0) {
                throw "$Path has unmatched JavaScript '$Token' at line $Line, column $($Column + 1)"
            }
            $Top = $Stack[$Stack.Count - 1]
            $ExpectedOpener = $Closers[$Token]
            if ($Top.Token -ne $ExpectedOpener) {
                throw "$Path has mismatched JavaScript '$Token' at line $Line, column $($Column + 1); opened '$($Top.Token)' at line $($Top.Line), column $($Top.Column)"
            }
            $Stack.RemoveAt($Stack.Count - 1)
        }

        if (-not [char]::IsWhiteSpace($Ch)) {
            $PreviousSignificant = $Token
        }
        $Index += 1
        $Column += 1
    }

    if ($State -eq "blockComment") {
        throw "$Path has unterminated JavaScript block comment"
    }
    if ($State -eq "singleString" -or $State -eq "doubleString" -or $State -eq "templateString") {
        throw "$Path has unterminated JavaScript string starting at line $StringStartLine, column $StringStartColumn"
    }
    if ($State -eq "regexLiteral") {
        throw "$Path has unterminated JavaScript regular expression literal starting at line $StringStartLine, column $StringStartColumn"
    }
    if ($Stack.Count -gt 0) {
        $Top = $Stack[$Stack.Count - 1]
        throw "$Path has unclosed JavaScript '$($Top.Token)' opened at line $($Top.Line), column $($Top.Column)"
    }
}

function Invoke-JavaScriptSyntaxCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Paths,

        [Parameter(Mandatory = $true)]
        [string] $Label
    )

    $Node = Get-Command node -ErrorAction SilentlyContinue
    if ($null -ne $Node) {
        $NodeUsable = $true
        try {
            & $Node.Source "--version" *> $null
            if ($LASTEXITCODE -ne 0) {
                throw "node --version failed with exit code $LASTEXITCODE"
            }
        } catch {
            Write-Host "Node found but not executable; running $Label JavaScript structural fallback check. $($_.Exception.Message)"
            $NodeUsable = $false
        }
        if ($NodeUsable) {
            foreach ($Path in $Paths) {
                Invoke-Native $Node.Source "--check" $Path
            }
            return
        }
    } else {
        Write-Host "Node not found; running $Label JavaScript structural fallback check."
    }

    foreach ($Path in $Paths) {
        Assert-JavaScriptStructuralBalance -Path $Path
    }
    Write-Host "$Label JavaScript structural fallback check passed."
}

function Assert-VscodeExtensionContract {
    $ExtensionRoot = Join-Path $RepoRoot "tools\vscode-englang"
    $PackageJsonPath = Join-Path $ExtensionRoot "package.json"
    $ExtensionJsPath = Join-Path $ExtensionRoot "extension.js"
    $ArtifactOpenersPath = Join-Path $ExtensionRoot "artifactOpeners.js"
    $CommandHandlersPath = Join-Path $ExtensionRoot "commandHandlers.js"
    $DecorationsPath = Join-Path $ExtensionRoot "decorations.js"
    $CompletionProviderPath = Join-Path $ExtensionRoot "completionProvider.js"
    $DiagnosticsProviderPath = Join-Path $ExtensionRoot "diagnosticsProvider.js"
    $HoverProviderPath = Join-Path $ExtensionRoot "hoverProvider.js"
    $CodeActionProviderPath = Join-Path $ExtensionRoot "codeActionProvider.js"
    $FoldingRangeProviderPath = Join-Path $ExtensionRoot "foldingRangeProvider.js"
    $FormattingProviderPath = Join-Path $ExtensionRoot "formattingProvider.js"
    $NavigationProvidersPath = Join-Path $ExtensionRoot "navigationProviders.js"
    $SemanticTokensProviderPath = Join-Path $ExtensionRoot "semanticTokensProvider.js"
    $LocalCodeActionsPath = Join-Path $ExtensionRoot "localCodeActions.js"
    $LspCodeActionsPath = Join-Path $ExtensionRoot "lspCodeActions.js"
    $LspKindsPath = Join-Path $ExtensionRoot "lspKinds.js"
    $LspNavigationPath = Join-Path $ExtensionRoot "lspNavigation.js"
    $LspRangesPath = Join-Path $ExtensionRoot "lspRanges.js"
    $LspRequestsPath = Join-Path $ExtensionRoot "lspRequests.js"
    $LspSemanticTokensPath = Join-Path $ExtensionRoot "lspSemanticTokens.js"
    $ArtifactRegistryPath = Join-Path $ExtensionRoot "artifactRegistry.js"
    $EditorMetadataLoaderPath = Join-Path $ExtensionRoot "editorMetadata.js"
    $ExecutionProfilesPath = Join-Path $ExtensionRoot "executionProfiles.js"
    $ModuleStatusPath = Join-Path $ExtensionRoot "moduleStatus.js"
    $RuntimeDiscoveryPath = Join-Path $ExtensionRoot "runtimeDiscovery.js"
    $ReviewPanelRendererPath = Join-Path $ExtensionRoot "reviewPanelRenderer.js"
    $SnippetsPath = Join-Path $ExtensionRoot "snippets\eng.json"
    $LspSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\lib.rs"
    $LspCliSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\main.rs"
    $CompilerLexerPath = Join-Path $RepoRoot "crates\eng_compiler\src\lexer.rs"
    $EditorMetadataPath = Join-Path $ExtensionRoot "generated\editor\englang-editor-metadata.json"
    $SemanticLegendPath = Join-Path $ExtensionRoot "generated\editor\englang-semantic-legend.json"
    $CompletionsPath = Join-Path $ExtensionRoot "generated\editor\englang-completions.json"
    $SyntaxCatalogPath = Join-Path $ExtensionRoot "generated\editor\englang-syntax.json"
    $TokenScopesDocPath = Join-Path $RepoRoot "docs\internal\editor\token_scopes.md"
    $DevScriptPath = Join-Path $RepoRoot "scripts\dev.ps1"
    $VscodeReadmePath = Join-Path $ExtensionRoot "README.md"
    $VscodeDarkThemePath = Join-Path $ExtensionRoot "themes\englang-dark-color-theme.json"
    $VscodeLightThemePath = Join-Path $ExtensionRoot "themes\englang-light-color-theme.json"
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
    if (-not (Test-Path $ArtifactOpenersPath)) {
        throw "missing VS Code artifact opener helpers at $ArtifactOpenersPath"
    }
    if (-not (Test-Path $CommandHandlersPath)) {
        throw "missing VS Code command handlers at $CommandHandlersPath"
    }
    if (-not (Test-Path $DecorationsPath)) {
        throw "missing VS Code decoration controller at $DecorationsPath"
    }
    if (-not (Test-Path $CompletionProviderPath)) {
        throw "missing VS Code completion provider at $CompletionProviderPath"
    }
    if (-not (Test-Path $DiagnosticsProviderPath)) {
        throw "missing VS Code diagnostics provider at $DiagnosticsProviderPath"
    }
    if (-not (Test-Path $HoverProviderPath)) {
        throw "missing VS Code hover provider at $HoverProviderPath"
    }
    if (-not (Test-Path $CodeActionProviderPath)) {
        throw "missing VS Code code action provider at $CodeActionProviderPath"
    }
    if (-not (Test-Path $FoldingRangeProviderPath)) {
        throw "missing VS Code folding range provider at $FoldingRangeProviderPath"
    }
    if (-not (Test-Path $FormattingProviderPath)) {
        throw "missing VS Code formatting provider at $FormattingProviderPath"
    }
    if (-not (Test-Path $NavigationProvidersPath)) {
        throw "missing VS Code navigation providers at $NavigationProvidersPath"
    }
    if (-not (Test-Path $SemanticTokensProviderPath)) {
        throw "missing VS Code semantic tokens provider at $SemanticTokensProviderPath"
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
    if (-not (Test-Path $LspRequestsPath)) {
        throw "missing VS Code LSP request bridge at $LspRequestsPath"
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
    if (-not (Test-Path $RuntimeDiscoveryPath)) {
        throw "missing VS Code runtime discovery helper at $RuntimeDiscoveryPath"
    }
    if (-not (Test-Path $ReviewPanelRendererPath)) {
        throw "missing VS Code review panel renderer at $ReviewPanelRendererPath"
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
    if (-not (Test-Path $CompilerLexerPath)) {
        throw "missing compiler lexer source at $CompilerLexerPath"
    }
    foreach ($RequiredMetadataPath in @($EditorMetadataPath, $SemanticLegendPath, $CompletionsPath, $SyntaxCatalogPath)) {
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
    $VscodeDarkTheme = Get-Content -LiteralPath $VscodeDarkThemePath -Raw | ConvertFrom-Json
    $VscodeLightTheme = Get-Content -LiteralPath $VscodeLightThemePath -Raw | ConvertFrom-Json
    $TokenScopesDoc = Get-Content -LiteralPath $TokenScopesDocPath -Raw
    $CompilerLexerSource = Get-Content -LiteralPath $CompilerLexerPath -Raw
    $DevScriptSource = Get-Content -LiteralPath $DevScriptPath -Raw
    $VscodeReadmeSource = Get-Content -LiteralPath $VscodeReadmePath -Raw
    $NativeIdeHowtoSource = Get-Content -LiteralPath $NativeIdeHowtoPath -Raw
    $UserGuideSource = Get-Content -LiteralPath $UserGuidePath -Raw
    $FeatureMaturitySource = Get-Content -LiteralPath $FeatureMaturityPath -Raw
    $MainInternalStatusSource = Get-Content -LiteralPath $MainInternalStatusPath -Raw
    $CurrentStatusSource = Get-Content -LiteralPath $CurrentStatusPath -Raw
    $CurrentTracksSource = Get-Content -LiteralPath $CurrentTracksPath -Raw
    if (-not $VscodeReadmeSource.Contains("completion_items") -or $VscodeReadmeSource.Contains("completion_seed") -or -not $VscodeReadmeSource.Contains("static completion fallback") -or -not $VscodeReadmeSource.Contains("syntax_catalog.legacy_unit_aliases") -or -not $VscodeReadmeSource.Contains("syntax_catalog.legacy_workflow_builtin_aliases") -or -not $VscodeReadmeSource.Contains("syntax_catalog.legacy_workflow_option_aliases") -or -not $VscodeReadmeSource.Contains("syntax_catalog.model_fields") -or -not $VscodeReadmeSource.Contains("syntax_catalog.prediction_table_fields") -or -not $VscodeReadmeSource.Contains("syntax_catalog.coverage_result_fields") -or -not $VscodeReadmeSource.Contains("syntax_catalog.table_fields") -or -not $VscodeReadmeSource.Contains("public member API") -or -not $VscodeReadmeSource.Contains("runtime-backed public fields") -or -not $VscodeReadmeSource.Contains("editor-only placeholders") -or -not $VscodeReadmeSource.Contains("highlight-only compatibility aliases")) {
        throw "VS Code README must document completion_items as the editor metadata completion catalog, public member field catalogs, and legacy aliases as highlight-only metadata without completion_seed"
    }
    if (-not $VscodeReadmeSource.Contains("overlapping highlight ranges") -or -not $VscodeReadmeSource.Contains("line overlap rows")) {
        throw "VS Code README must document highlight overlap rows in user-facing terms"
    }
    if (-not $VscodeReadmeSource.Contains("status bar") -or -not $VscodeReadmeSource.Contains("EngLang Problems mode") -or -not $VscodeReadmeSource.Contains("error/warning/info/hint counts")) {
        throw "VS Code README must document the EngLang Problems status bar in user-facing terms"
    }
    if (-not $VscodeReadmeSource.Contains("The refresh follows the") -or -not $VscodeReadmeSource.Contains("file mode checks the saved file") -or -not $VscodeReadmeSource.Contains("live mode can") -or -not $VscodeReadmeSource.Contains("current unsaved buffer")) {
        throw "VS Code README must document that Refresh Problems follows the selected diagnostics mode"
    }
    if (-not $VscodeReadmeSource.Contains("the underlined source") -or -not $VscodeReadmeSource.Contains("full source line") -or -not $VscodeReadmeSource.Contains("copy-ready reports")) {
        throw "VS Code README must document problem inspector source text payloads"
    }
    if (-not $VscodeReadmeSource.Contains("EngLang: Copy Problem at Cursor") -or -not $VscodeReadmeSource.Contains("nearest same-line diagnostic payload") -or -not $VscodeReadmeSource.Contains("clipboard")) {
        throw "VS Code README must document the copy-ready problem cursor command"
    }
    if (-not $VscodeReadmeSource.Contains("EngLang: Copy Highlight Token at Cursor") -or -not $VscodeReadmeSource.Contains("same-line role-aware highlight token payload") -or -not $VscodeReadmeSource.Contains("nearest same-line highlight token payload")) {
        throw "VS Code README must document the copy-ready highlight cursor command"
    }
    if (-not $VscodeReadmeSource.Contains("cursor diagnostic inspection and") -or -not $VscodeReadmeSource.Contains("copy commands") -or -not $VscodeReadmeSource.Contains("highlight inspection and copy commands") -or -not $VscodeReadmeSource.Contains("native workflow source/docs")) {
        throw "VS Code README must document copy commands in Tooling Status discoverability wording"
    }
    foreach ($ForbiddenPublicMemberCatalogWording in @("seed-only suggestions", "non-executable placeholder suggestions")) {
        if ($VscodeReadmeSource.Contains($ForbiddenPublicMemberCatalogWording)) {
            throw "VS Code README must not describe public member catalogs as $ForbiddenPublicMemberCatalogWording"
        }
    }
    if ($Package.name -ne "englang") {
        throw "VS Code extension package name must be englang"
    }
    if ($Package.main -ne "./extension.js") {
        throw "VS Code extension main must be ./extension.js"
    }
    $PackageDescription = [string]$Package.description
    if ($PackageDescription -match "run commands" -or -not $PackageDescription.Contains("program execution")) {
        throw "VS Code extension description must say program execution instead of run commands"
    }
    if ($DevScriptSource -notmatch '<Description[^>]*>EngLang editor tooling with diagnostics, hover, completion, and program execution\.</Description>' -or $DevScriptSource -match '<Description[^>]*>EngLang editor tooling with diagnostics, hover, completion, and run commands\.</Description>') {
        throw "VS Code generated VSIX manifest description source must say program execution instead of run commands"
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
    $BuildGrammarPath = Join-Path $ExtensionRoot "scripts\build-grammar.ps1"
    if (-not (Test-Path $BuildGrammarPath)) {
        throw "VS Code extension missing grammar build script at $BuildGrammarPath"
    }
    $LanguageConfigurationPath = Join-Path $ExtensionRoot "language-configuration.json"
    if (-not (Test-Path $LanguageConfigurationPath)) {
        throw "VS Code extension missing language configuration at $LanguageConfigurationPath"
    }
    $LanguageConfiguration = Get-Content -LiteralPath $LanguageConfigurationPath -Raw | ConvertFrom-Json
    if ($LanguageConfiguration.comments.lineComment -ne "#") {
        throw "VS Code extension language configuration must keep # as line comment"
    }
    if ($LanguageConfiguration.indentationRules.increaseIndentPattern -ne '^.*\{\s*(?:(#|//).*)?$') {
        throw "VS Code extension language configuration must indent after block openers with # or // trailing comments"
    }
    if ($LanguageConfiguration.indentationRules.decreaseIndentPattern -ne '^\s*\}') {
        throw "VS Code extension language configuration must outdent block closers"
    }
    if ($LanguageConfiguration.wordPattern -ne '(latin-hypercube)|(-?\d+(?:\.\d+)?)|([A-Za-z%][A-Za-z0-9%]*(?:\^[0-9]+)?(?:/[A-Za-z0-9%]+(?:\^[0-9]+)?)+)|([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)') {
        throw "VS Code extension language configuration must treat dotted EngLang symbols, slash/exponent units, and hyphenated workflow builtins as words"
    }
    $Snippets = Get-Content -LiteralPath $SnippetsPath -Raw | ConvertFrom-Json
    foreach ($RequiredStaticSnippet in @(
        @{ Name = "Regression prediction table"; Tokens = @("train regression", "model_card", "evaluate", "predict") },
        @{ Name = "System model"; Tokens = @("system", "state", "equation energy_balance") },
        @{ Name = "Domain ports"; Tokens = @("domain", "package", "version", "connect") }
    )) {
        $SnippetProperty = $Snippets.PSObject.Properties[$RequiredStaticSnippet.Name]
        if ($null -eq $SnippetProperty) {
            throw "VS Code static snippets missing native snippet $($RequiredStaticSnippet.Name)"
        }
        $SnippetBody = (@($SnippetProperty.Value.body) -join "`n")
        foreach ($RequiredSnippetToken in $RequiredStaticSnippet.Tokens) {
            if (-not $SnippetBody.Contains($RequiredSnippetToken)) {
                throw "VS Code static snippet $($RequiredStaticSnippet.Name) missing token $RequiredSnippetToken"
            }
        }
    }
    foreach ($RequiredVscodeInstallPattern in @(
        '(?m)^\s+"vscode-status"\s*\{\s*Invoke-VscodeStatus\s*\}',
        '(?m)^\s+"vscode-package"\s*\{\s*Invoke-VscodePackage\s*\}',
        '(?m)^\s+"vscode-install"\s*\{\s*Invoke-VscodeInstall\s*\}',
        '(?m)^\s+\.\\dev\.bat vscode-status  Show local VS Code extension install/package status\s*$',
        '(?m)^\s+\.\\dev\.bat vscode-package Build a local installable VS Code extension VSIX\s*$',
        '(?m)^\s+\.\\dev\.bat vscode-install Build and install the EngLang VS Code extension with the code CLI\s*$'
    )) {
        if ($DevScriptSource -notmatch $RequiredVscodeInstallPattern) {
            throw "dev wrapper missing VS Code local install contract pattern $RequiredVscodeInstallPattern"
        }
    }
    foreach ($RequiredVscodeInstallDocToken in @(
        ".\dev.bat vscode-status",
        ".\dev.bat vscode-install",
        ".\dev.bat vscode-package",
        "dist\local-vscode\tools\englang-vscode-<version>.vsix",
        "Extensions: Install from VSIX...",
        "Close all VS Code windows before reinstalling EngLang",
        "Install freshness",
        "Package freshness"
    )) {
        if (-not $VscodeReadmeSource.Contains($RequiredVscodeInstallDocToken)) {
            throw "VS Code README missing local install token $RequiredVscodeInstallDocToken"
        }
        if (-not $NativeIdeHowtoSource.Contains($RequiredVscodeInstallDocToken)) {
            throw "native IDE how-to missing VS Code install token $RequiredVscodeInstallDocToken"
        }
    }
    foreach ($RequiredVscodeInstallPreflightToken in @(
        "Assert-VscodeInstallPreflight",
        "Get-VscodeUserExtensionsDirectory",
        "Get-LocalVscodeVsixPath",
        "Get-InstalledVscodeEnglangExtensionPaths",
        "Get-RunningVscodeProcessSummaries",
        "Get-LocalVscodeVsixSummary",
        "Get-VscodeExtensionInstallSummary",
        "Get-VscodeExtensionInstallUpdatedTime",
        "Get-VscodeExtensionFreshnessSummary",
        "Get-LatestVscodePackageInputTime",
        "Get-VscodePackageFreshnessSummary",
        "Format-VscodeTimestamp",
        "Package freshness",
        "rebuild available",
        "Install freshness",
        "update available",
        "Format-ByteSize",
        'version $(Get-WorkspaceVersion)',
        'updated $Updated',
        "package.json unreadable",
        "Invoke-NativeInDirectory",
        "--user-data-dir",
        "--extensions-dir",
        "vscode-install",
        "Close all VS Code windows before reinstalling EngLang",
        "Existing built VSIX",
        "vscode-package"
    )) {
        if (-not $DevScriptSource.Contains($RequiredVscodeInstallPreflightToken)) {
            throw "dev wrapper missing VS Code install preflight token $RequiredVscodeInstallPreflightToken"
        }
    }
    $DocCommentEnterRule = @($LanguageConfiguration.onEnterRules) | Where-Object {
        $_.beforeText -eq "^\s*///.*$" -and $_.action.appendText -eq "/// "
    } | Select-Object -First 1
    if ($null -eq $DocCommentEnterRule) {
        throw "VS Code extension language configuration must continue /// doc comments on Enter"
    }
    $GrammarSource = Get-Content -LiteralPath $GrammarPath -Raw
    $GrammarTemplateSource = Get-Content -LiteralPath $GrammarSourcePath -Raw
    $BuildGrammarSource = Get-Content -LiteralPath $BuildGrammarPath -Raw
    foreach ($RequiredGrammarToken in @(
        "read", "json", "toml", "render", "template", "open", "sqlite",
        "post", "check", "coverage", "sample", "lhs", "uniform",
        "require_one", "regression_table", "meta.workflow.train-regression.englang", "support.namespace.module.englang",
        "materialize", "apply", "collect", "case_id", "output_root", "resume", "step",
        "run_case", "train_test_split", "regression", "predict", "model_card",
        "CsvFile", "JsonFile", "DirectoryPath", "DimensionlessNumber",
        "expected_outputs", "artifact_kind", "cache_key", "allow_failure",
        "OutputManifest", "metadata_ready", "backend", "display_unit",
        "variable_scale", "consistency_tolerance", "meta.workflow.with-block.englang",
        "fixture",
        "variable.parameter.function.englang", "storage.type.function.englang",
        "storage.type.test.englang", "storage.type.interface-member.englang", "storage.modifier.englang",
        "constant.character.escape.englang", "variable.other.interpolation.englang", "punctuation.separator.parameter.englang",
        "punctuation.separator.format.englang",
        "constant.numeric.format.englang", "constant.other.unit.format.englang", "entity.name.function.call.englang",
        "storage.modifier.state.englang", "storage.modifier.input.englang", "storage.modifier.parameter.englang",
        "storage.modifier.output.englang", "storage.modifier.operator.englang",
        "variable.other.state.englang", "variable.other.input.englang", "variable.other.output.englang",
        "variable.other.parameter.englang", "entity.name.function.solver.englang", "meta.declaration.equation.englang",
        "keyword.control.import.englang", "keyword.control.validation.englang", "keyword.control.workflow.englang",
        "keyword.control.side-effect.englang", "keyword.control.external-boundary.englang"
    )) {
        if (-not $GrammarSource.Contains($RequiredGrammarToken)) {
            throw "VS Code grammar missing token $RequiredGrammarToken"
        }
    }
    foreach ($RequiredGrammarPlaceholder in @(
        "{{KEYWORD_GROUP_IMPORT}}", "{{KEYWORD_GROUP_DEPRECATED}}", "{{KEYWORD_GROUP_DECLARATION}}",
        "{{KEYWORD_GROUP_FUNCTION}}", "{{KEYWORD_GROUP_TEST}}", "{{KEYWORD_GROUP_BLOCK}}",
        "{{KEYWORD_GROUP_MODIFIER}}", "{{KEYWORD_GROUP_REPORT}}", "{{KEYWORD_GROUP_VALIDATION}}",
        "{{KEYWORD_GROUP_SIDE_EFFECT}}",
        "{{KEYWORD_GROUP_EXTERNAL_BOUNDARY}}", "{{KEYWORD_GROUP_SOLVER}}", "{{KEYWORD_GROUP_WORKFLOW}}",
        "{{WORKFLOW_STATUS_LITERALS}}"
    )) {
        if (-not $GrammarTemplateSource.Contains($RequiredGrammarPlaceholder)) {
            throw "VS Code source grammar missing generated placeholder $RequiredGrammarPlaceholder"
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
    if (-not $BuildGrammarSource.Contains("legacy_workflow_builtin_aliases") -or -not $BuildGrammarSource.Contains("LegacyWorkflowBuiltinAliases") -or -not $BuildGrammarSource.Contains("legacy_workflow_option_aliases") -or -not $BuildGrammarSource.Contains("LegacyWorkflowOptionAliases")) {
        throw "VS Code grammar build must read compatibility-only workflow builtin and option highlight aliases from generated metadata"
    }
    if (-not $BuildGrammarSource.Contains("operator_words") -or -not $BuildGrammarSource.Contains("GrammarOnlyOperatorWordAliases") -or -not $BuildGrammarSource.Contains("{{OPERATOR_WORDS}}")) {
        throw "VS Code grammar build must generate operator words from editor metadata while preserving compatibility aliases"
    }
    if (-not $BuildGrammarSource.Contains("keyword_groups") -or -not $BuildGrammarSource.Contains("{{KEYWORD_GROUP_WORKFLOW}}")) {
        throw "VS Code grammar build must generate keyword groups from editor metadata"
    }
    if (-not $BuildGrammarSource.Contains("workflow_status_literals") -or -not $BuildGrammarSource.Contains("{{WORKFLOW_STATUS_LITERALS}}")) {
        throw "VS Code grammar build must generate workflow status literals from editor metadata"
    }
    & (Join-Path $ExtensionRoot "scripts\build-grammar.ps1") -ExtensionRoot $ExtensionRoot -Check
    & (Join-Path $ExtensionRoot "scripts\test-grammar.ps1") -ExtensionRoot $ExtensionRoot
    & (Join-Path $ExtensionRoot "scripts\build-editor-metadata.ps1") -ExtensionRoot $ExtensionRoot -Check
    $Commands = @($Package.contributes.commands | ForEach-Object { $_.command })
    if ($Commands -contains "englang.switchProblemsSource") {
        throw "VS Code extension must not expose legacy switchProblemsSource command in package metadata"
    }
    foreach ($Required in @(
        "englang.checkFile",
        "englang.refreshProblems",
        "englang.runFile",
        "englang.runExample",
        "englang.switchProfile",
        "englang.switchDiagnosticsMode",
        "englang.showToolingStatus",
        "englang.showProblemAtCursor",
        "englang.copyProblemAtCursor",
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
        "englang.showSemanticTokensDebug",
        "englang.showSemanticTokenAtCursor",
        "englang.copySemanticTokenAtCursor"
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
        @{ Command = "englang.openReportSpec"; Text = "Last Run Report Source Data" },
        @{ Command = "englang.openOutputManifest"; Text = "Last Run Generated Output List" },
        @{ Command = "englang.openRunLog"; Text = "Last Run Log" },
        @{ Command = "englang.openStaticRunPlan"; Text = "Last Static Run Graph" },
        @{ Command = "englang.openRunPlan"; Text = "Last Run Graph" },
        @{ Command = "englang.openRunLock"; Text = "Last Run Reproducibility Lock" },
        @{ Command = "englang.openProcessResults"; Text = "Last Run Process Results" },
        @{ Command = "englang.openCacheManifest"; Text = "Last Run Cache Records" },
        @{ Command = "englang.openTestResults"; Text = "Last Run Test Results" },
        @{ Command = "englang.openPlotSpec"; Text = "Last Run Plot Data" },
        @{ Command = "englang.openPlotManifest"; Text = "Last Run Plot Output List" },
        @{ Command = "englang.openPlotSvg"; Text = "Last Run Plot SVG" },
        @{ Command = "englang.switchDiagnosticsMode"; Text = "Switch Diagnostics Mode" },
        @{ Command = "englang.refreshProblems"; Text = "Refresh Problems" },
        @{ Command = "englang.showToolingStatus"; Text = "Show Tooling Status" },
        @{ Command = "englang.showProblemAtCursor"; Text = "Inspect Problem at Cursor" },
        @{ Command = "englang.copyProblemAtCursor"; Text = "Copy Problem at Cursor" },
        @{ Command = "englang.showSemanticTokensDebug"; Text = "Inspect Highlight Tokens" },
        @{ Command = "englang.showSemanticTokenAtCursor"; Text = "Inspect Highlight Token at Cursor" },
        @{ Command = "englang.copySemanticTokenAtCursor"; Text = "Copy Highlight Token at Cursor" }
    )) {
        $Title = $CommandTitles[$RequiredTitle.Command]
        if ([string]::IsNullOrWhiteSpace($Title) -or -not $Title.Contains($RequiredTitle.Text)) {
            throw "VS Code command $($RequiredTitle.Command) must expose clearer title text containing '$($RequiredTitle.Text)'"
        }
    }
    $EditorContextMenu = @($Package.contributes.menus."editor/context")
    foreach ($RequiredEditorContextMenu in @("englang.refreshProblems", "englang.showProblemAtCursor", "englang.copyProblemAtCursor", "englang.showSemanticTokenAtCursor", "englang.copySemanticTokenAtCursor")) {
        $MenuItem = @($EditorContextMenu | Where-Object { $_.command -eq $RequiredEditorContextMenu }) | Select-Object -First 1
        if ($null -eq $MenuItem -or [string]$MenuItem.when -ne "editorLangId == englang") {
            throw "VS Code editor context menu must expose $RequiredEditorContextMenu only for EngLang editors"
        }
    }
    $Properties = $Package.contributes.configuration.properties
    foreach ($RequiredProperty in @("englang.runtimePath", "englang.lspPath", "englang.diagnosticsMode", "englang.executionProfile", "englang.lintOnSave", "englang.lintOnChange", "englang.semanticHighlighting.enabled", "englang.reviewRiskDecorations.enabled")) {
        if ($null -eq $Properties.$RequiredProperty) {
            throw "VS Code extension missing configuration property $RequiredProperty"
        }
    }
    $DiagnosticsModeDescription = [string]$Properties."englang.diagnosticsMode".description
    if ($DiagnosticsModeDescription -match "eng-cli|lsp-snapshot|snapshot path|metadata|Problems source") {
        throw "VS Code diagnosticsMode description must use user-facing wording, not implementation details"
    }
    $DiagnosticsModeEnumDescriptions = @($Properties."englang.diagnosticsMode".enumDescriptions)
    foreach ($DiagnosticsModeEnumDescription in $DiagnosticsModeEnumDescriptions) {
        if ([string]$DiagnosticsModeEnumDescription -match "eng-cli|lsp-snapshot|snapshot path|metadata|Problems source") {
            throw "VS Code diagnosticsMode enum descriptions must use user-facing wording, not implementation details"
        }
    }
    if ($null -ne $Properties."englang.problemsSource") {
        throw "VS Code extension must not expose legacy problemsSource in Settings; keep it as a code-only compatibility alias"
    }
    if ($null -ne $Properties."englang.diagnosticsBackend") {
        throw "VS Code extension must not expose deprecated diagnosticsBackend in Settings; keep it as a code-only compatibility alias"
    }
    $ExecutionProfileDescription = [string]$Properties."englang.executionProfile".description
    if ($ExecutionProfileDescription -match "run commands" -or -not $ExecutionProfileDescription.Contains("program runs")) {
        throw "VS Code executionProfile description must say program runs instead of run commands"
    }
    $RuntimePathDescription = [string]$Properties."englang.runtimePath".description
    if (-not $RuntimePathDescription.Contains("check/run tool")) {
        throw "VS Code runtimePath description must describe the check/run tool"
    }
    $LintOnSaveDescription = [string]$Properties."englang.lintOnSave".description
    if (-not $LintOnSaveDescription.Contains("saved-file EngLang Problems diagnostics")) {
        throw "VS Code lintOnSave description must explain saved-file Problems diagnostics"
    }
    $LintOnChangeDescription = [string]$Properties."englang.lintOnChange".description
    if ($LintOnChangeDescription -match "eng-lsp|snapshot") {
        throw "VS Code lintOnChange description must avoid editor-service implementation details"
    }
    if (-not $LintOnChangeDescription.Contains("diagnostics mode is live") -or -not $LintOnChangeDescription.Contains("VS Code Problems diagnostics")) {
        throw "VS Code lintOnChange description must say it applies to live VS Code Problems diagnostics"
    }
    $LspPathDescription = [string]$Properties."englang.lspPath".description
    if ($LspPathDescription -match "editor service") {
        throw "VS Code lspPath description must describe live editor features instead of editor-service internals"
    }
    if (-not $LspPathDescription.Contains("live editor tool")) {
        throw "VS Code lspPath description must describe the live editor tool"
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
        "persistent LSP integration",
        "active runtime/LSP paths",
        "LSP-backed"
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
    foreach ($ForbiddenNativeIdeHowtoWording in @(
        "semantic-token legend",
        "semantic token type/modifiers",
        "LSP editor metadata",
        "LSP editor syntax catalog",
        "compiler-backed semantic tokens",
        "token ranges",
        "lint toggles"
    )) {
        if ($NativeIdeHowtoSource.Contains($ForbiddenNativeIdeHowtoWording)) {
            throw "Native IDE user how-to must describe highlight UI in user-facing terms: $ForbiddenNativeIdeHowtoWording"
        }
    }
    if (-not $NativeIdeHowtoSource.Contains("saved-file/live Problems diagnostics toggles")) {
        throw "Native IDE user how-to must describe VS Code diagnostics toggles with Problems wording"
    }
    foreach ($ForbiddenVscodeReadmeWording in @(
        "semantic token modifier and TextMate fallback scope metadata",
        "semantic token modifiers and TextMate fallback scopes",
        "raw semantic-token payload",
        "semantic-token mapping rules"
    )) {
        if ($VscodeReadmeSource.Contains($ForbiddenVscodeReadmeWording)) {
            throw "VS Code README must describe highlighting in user-facing terms: $ForbiddenVscodeReadmeWording"
        }
    }
    $RiskDecorationDescription = [string]$Properties."englang.reviewRiskDecorations.enabled".description
    if ($RiskDecorationDescription -notmatch "review risks") {
        throw "VS Code reviewRiskDecorations setting must describe review-risk markers"
    }
    $SemanticModifiers = @($Package.contributes.semanticTokenModifiers | ForEach-Object { $_.id })
    foreach ($RequiredSemanticModifier in @(
        "unit", "quantity", "axis", "timeseries", "uncertain",
        "sideEffect", "external", "validation", "report", "solver",
        "planned", "internal", "riskHigh", "riskMedium", "state", "input", "output",
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
    foreach ($RequiredTokenScopeDocToken in @(
        "syntax_catalog.workflow_status_literals",
        "syntax_catalog.legacy_workflow_builtin_aliases",
        "syntax_catalog.legacy_workflow_option_aliases",
        "syntax_catalog.keywords",
        "syntax_catalog.keyword_groups",
        "syntax_catalog.constants",
        "syntax_catalog.operator_words",
        "syntax_catalog.units",
        "syntax_catalog.legacy_unit_aliases",
        "compiler-owned unit catalog",
        "native IDE lexical fallback consumes",
        "status ==",
        "status !=",
        "status ="
    )) {
        if (-not $TokenScopesDoc.Contains($RequiredTokenScopeDocToken)) {
            throw "editor token scope contract missing workflow status literal token $RequiredTokenScopeDocToken"
        }
    }

    $SemanticScopeRule = @($Package.contributes.semanticTokenScopes | Where-Object { $_.language -eq "englang" }) | Select-Object -First 1
    if ($null -eq $SemanticScopeRule) {
        throw "VS Code extension missing englang semantic token scope mappings"
    }
    foreach ($RequiredTokenScope in @(
        "type", "type.static", "class.declaration", "class.defaultLibrary", "class.static", "comment", "comment.documentation",
        "function", "function.static", "interface", "interface.static", "method", "method.declaration", "method.static", "namespace", "namespace.static", "variable.local", "function.declaration",
        "function.definition", "function.report", "interface.declaration", "interface.defaultLibrary",
        "keyword", "keyword.declaration", "keyword.state", "keyword.input", "keyword.output", "keyword.local", "keyword.static", "modifier", "modifier.static", "namespace.declaration", "number",
        "parameter", "parameter.readonly", "parameter.static", "property", "property.declaration", "property.readonly", "property.static", "variable",
        "variable.declaration", "variable.defaultLibrary", "variable.readonly", "variable.deprecated", "variable.static",
        "type.unit", "type.quantity", "property.unit", "variable.quantity", "function.external",
        "method.sideEffect", "property.sideEffect", "variable.validation", "variable.report",
        "keyword.sideEffect", "keyword.external", "keyword.validation",
        "keyword.report", "keyword.solver", "function.solver",
        "property.external", "property.solver", "keyword.deprecated", "property.deprecated", "class.deprecated", "variable.state",
        "variable.input", "parameter.input", "variable.output", "variable.riskHigh", "keyword.riskHigh", "class.riskHigh",
        "property.riskHigh", "variable.riskMedium", "keyword.riskMedium", "class.riskMedium",
        "property.riskMedium", "variable.model", "variable.db", "keyword.db", "function.db", "method.db", "property.db",
        "function.model", "keyword.model", "function.defaultLibrary", "function.timeseries", "namespace.defaultLibrary",
        "namespace.imported", "namespace.internal", "namespace.planned", "type.axis",
        "variable.cache", "keyword.cache", "function.cache", "method.cache", "property.cache",
        "keyword.uncertain", "keyword.workflowStep", "function.workflowStep"
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
    $RequiredSemanticObservedFallbacks = @{
        "function" = @(
            "entity.name.function.call.englang",
            "entity.name.function.englang"
        )
        "interface" = @(
            "entity.name.type.englang",
            "support.type.englang"
        )
        "method" = @(
            "entity.name.function.call.englang",
            "entity.name.function.englang"
        )
        "method.declaration" = @(
            "entity.name.function.englang"
        )
        "variable.cache" = @(
            "keyword.control.workflow.englang",
            "variable.other.definition.englang"
        )
        "function.cache" = @(
            "keyword.control.workflow.englang",
            "support.function.builtin.englang"
        )
        "method.cache" = @(
            "keyword.control.workflow.englang",
            "entity.name.function.englang"
        )
        "property.cache" = @(
            "keyword.control.workflow.englang",
            "variable.other.property.englang"
        )
        "variable.db" = @(
            "keyword.control.external-boundary.englang",
            "variable.other.definition.englang"
        )
        "function.db" = @(
            "keyword.control.external-boundary.englang",
            "support.function.builtin.englang"
        )
        "method.db" = @(
            "keyword.control.external-boundary.englang",
            "entity.name.function.englang"
        )
        "property.db" = @(
            "keyword.control.external-boundary.englang",
            "variable.other.property.englang"
        )
        "namespace" = @(
            "support.namespace.module.englang"
        )
        "variable.local" = @(
            "variable.other.local.englang",
            "variable.other.definition.englang"
        )
        "property.readonly" = @(
            "variable.other.public-member.englang",
            "variable.other.property.englang"
        )
        "variable.deprecated" = @(
            "keyword.control.deprecated.englang",
            "variable.other.definition.englang",
            "variable.other.local.englang"
        )
        "property.deprecated" = @(
            "keyword.control.deprecated.englang",
            "variable.other.public-member.englang",
            "variable.other.property.englang"
        )
    }
    foreach ($Selector in $RequiredSemanticObservedFallbacks.Keys) {
        $ScopeProperty = $SemanticScopeRule.scopes.PSObject.Properties[$Selector]
        if ($null -eq $ScopeProperty) {
            throw "VS Code extension missing observed semantic token scope mapping $Selector"
        }
        $ScopeValues = @($ScopeProperty.Value)
        foreach ($RequiredFallbackScope in $RequiredSemanticObservedFallbacks[$Selector]) {
            if ($ScopeValues -notcontains $RequiredFallbackScope) {
                throw "VS Code extension observed semantic token scope mapping $Selector missing fallback scope $RequiredFallbackScope"
            }
        }
    }
    $RequiredSemanticKeywordFallbacks = @{
        "keyword.declaration" = @(
            "storage.type.declaration.englang",
            "storage.type.function.englang",
            "storage.type.test.englang",
            "storage.type.block.englang",
            "storage.type.interface-member.englang",
            "storage.modifier.state.englang",
            "storage.modifier.input.englang",
            "storage.modifier.parameter.englang",
            "storage.modifier.output.englang",
            "storage.modifier.operator.englang",
            "storage.modifier.englang"
        )
        "keyword.state" = @(
            "storage.modifier.state.englang",
            "storage.modifier.englang",
            "variable.other.state.englang"
        )
        "keyword.input" = @(
            "storage.modifier.input.englang",
            "storage.modifier.englang",
            "variable.other.input.englang"
        )
        "keyword.output" = @(
            "storage.modifier.output.englang",
            "storage.modifier.englang",
            "variable.other.output.englang"
        )
        "modifier" = @(
            "storage.modifier.englang",
            "storage.modifier.state.englang",
            "storage.modifier.input.englang",
            "storage.modifier.parameter.englang",
            "storage.modifier.output.englang",
            "storage.modifier.operator.englang",
            "storage.modifier.schema.englang"
        )
        "modifier.static" = @(
            "storage.modifier.englang",
            "storage.modifier.state.englang",
            "storage.modifier.input.englang",
            "storage.modifier.parameter.englang",
            "storage.modifier.output.englang",
            "storage.modifier.operator.englang",
            "storage.modifier.schema.englang"
        )
        "keyword.local" = @(
            "storage.type.block.englang",
            "keyword.control.workflow.englang"
        )
        "keyword.defaultLibrary" = @(
            "keyword.control.workflow.englang",
            "keyword.operator.word.englang",
            "keyword.control.validation.englang",
            "keyword.control.report.englang",
            "keyword.control.solver.englang",
            "constant.language.englang"
        )
        "keyword.workflowStep" = @(
            "keyword.control.workflow.englang",
            "keyword.operator.word.englang",
            "keyword.control.validation.englang",
            "constant.language.englang",
            "support.function.builtin.englang"
        )
        "keyword.model" = @(
            "support.function.builtin.englang",
            "keyword.control.workflow.englang",
            "keyword.operator.word.englang",
            "constant.language.englang"
        )
        "keyword.timeseries" = @(
            "keyword.control.workflow.englang",
            "keyword.control.validation.englang",
            "keyword.operator.word.englang"
        )
        "keyword.validation" = @(
            "keyword.control.validation.englang",
            "keyword.operator.word.englang",
            "constant.language.englang"
        )
        "keyword.report" = @(
            "keyword.control.report.englang",
            "keyword.operator.word.englang"
        )
        "keyword.solver" = @(
            "keyword.control.solver.englang",
            "keyword.operator.word.englang",
            "constant.language.englang"
        )
        "keyword.external" = @(
            "keyword.control.external-boundary.englang",
            "keyword.operator.word.englang"
        )
        "keyword.sideEffect" = @(
            "keyword.control.side-effect.englang",
            "keyword.operator.word.englang"
        )
        "keyword.cache" = @(
            "keyword.control.workflow.englang",
            "keyword.operator.word.englang",
            "constant.language.englang"
        )
        "keyword.uncertain" = @(
            "constant.language.englang",
            "keyword.operator.word.englang"
        )
        "keyword.db" = @(
            "keyword.control.external-boundary.englang",
            "keyword.operator.word.englang",
            "constant.language.englang"
        )
    }
    foreach ($Selector in $RequiredSemanticKeywordFallbacks.Keys) {
        $ScopeProperty = $SemanticScopeRule.scopes.PSObject.Properties[$Selector]
        if ($null -eq $ScopeProperty) {
            throw "VS Code extension missing semantic token scope mapping $Selector"
        }
        $ScopeValues = @($ScopeProperty.Value)
        foreach ($RequiredFallbackScope in $RequiredSemanticKeywordFallbacks[$Selector]) {
            if ($ScopeValues -notcontains $RequiredFallbackScope) {
                throw "VS Code extension semantic token scope mapping $Selector missing fallback scope $RequiredFallbackScope"
            }
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
    function Get-SemanticThemeColorKey {
        param($SemanticColorValue)
        if ($null -eq $SemanticColorValue) {
            return ""
        }
        if ($SemanticColorValue -is [string]) {
            return [string]$SemanticColorValue
        }
        if ($null -ne $SemanticColorValue.foreground) {
            return [string]$SemanticColorValue.foreground
        }
        return ($SemanticColorValue | ConvertTo-Json -Compress)
    }
    $RoleColorFamilies = @(
        @{ Label = "model"; Selectors = @("keyword.model", "function.model", "variable.model", "property.model") },
        @{ Label = "db"; Selectors = @("keyword.db", "function.db", "method.db", "variable.db", "property.db") },
        @{ Label = "cache"; Selectors = @("keyword.cache", "function.cache", "method.cache", "variable.cache", "property.cache") }
    )
    $ContributedThemes = @($Package.contributes.themes)
    foreach ($RequiredTheme in @(
        @{ Label = "EngLang Dark"; UiTheme = "vs-dark"; Path = "./themes/englang-dark-color-theme.json"; File = $VscodeDarkThemePath; Theme = $VscodeDarkTheme },
        @{ Label = "EngLang Light"; UiTheme = "vs"; Path = "./themes/englang-light-color-theme.json"; File = $VscodeLightThemePath; Theme = $VscodeLightTheme }
    )) {
        $ThemeContribution = @($ContributedThemes | Where-Object { $_.label -eq $RequiredTheme.Label }) | Select-Object -First 1
        if ($null -eq $ThemeContribution -or $ThemeContribution.uiTheme -ne $RequiredTheme.UiTheme -or $ThemeContribution.path -ne $RequiredTheme.Path) {
            throw "VS Code extension missing theme contribution $($RequiredTheme.Label)"
        }
        if (-not (Test-Path -LiteralPath $RequiredTheme.File -PathType Leaf)) {
            throw "VS Code extension theme file missing for $($RequiredTheme.Label): $($RequiredTheme.File)"
        }
        if ($RequiredTheme.Theme.semanticHighlighting -ne $true -or $null -eq $RequiredTheme.Theme.semanticTokenColors -or @($RequiredTheme.Theme.tokenColors).Count -lt 8) {
            throw "VS Code extension theme $($RequiredTheme.Label) must define TextMate and semantic token colors"
        }
        $RequiredThemeSelectors = @($SemanticScopeRule.scopes.PSObject.Properties.Name | Sort-Object -Unique)
        if ($RequiredThemeSelectors.Count -lt 100) {
            throw "VS Code extension semantic token scope map is unexpectedly small"
        }
        foreach ($RequiredThemeSelector in $RequiredThemeSelectors) {
            if ($null -eq $RequiredTheme.Theme.semanticTokenColors.PSObject.Properties[$RequiredThemeSelector]) {
                throw "VS Code extension theme $($RequiredTheme.Label) missing semantic token color $RequiredThemeSelector"
            }
        }
        $ThemeOnlySelectors = @($RequiredTheme.Theme.semanticTokenColors.PSObject.Properties.Name | Where-Object { $RequiredThemeSelectors -notcontains [string]$_ } | Sort-Object -Unique)
        if ($ThemeOnlySelectors.Count -gt 0) {
            throw "VS Code extension theme $($RequiredTheme.Label) has semantic token colors without fallback scope mappings: $($ThemeOnlySelectors -join ', ')"
        }
        foreach ($RoleColorFamily in $RoleColorFamilies) {
            $MissingRoleSelectors = @($RoleColorFamily.Selectors | Where-Object { $null -eq $RequiredTheme.Theme.semanticTokenColors.PSObject.Properties[[string]$_] })
            if ($MissingRoleSelectors.Count -gt 0) {
                throw "VS Code extension theme $($RequiredTheme.Label) missing semantic role color selector(s) for $($RoleColorFamily.Label): $($MissingRoleSelectors -join ', ')"
            }
            $RoleColorKeys = @($RoleColorFamily.Selectors | ForEach-Object {
                Get-SemanticThemeColorKey $RequiredTheme.Theme.semanticTokenColors.PSObject.Properties[[string]$_].Value
            } | Sort-Object -Unique)
            if ($RoleColorKeys.Count -lt 3) {
                throw "VS Code extension theme $($RequiredTheme.Label) must keep $($RoleColorFamily.Label) keyword/function/member semantic colors visually distinct"
            }
        }
    }
    if (-not $TokenScopesDoc.Contains("EngLang Dark") -or -not $TokenScopesDoc.Contains("EngLang Light") -or -not $VscodeReadmeSource.Contains("EngLang Dark") -or -not $VscodeReadmeSource.Contains("EngLang Light")) {
        throw "VS Code docs must mention the optional EngLang color themes"
    }
    foreach ($ForbiddenVscodeReadmeWording in @("promotion skeletons", "migration skeletons")) {
        if ($VscodeReadmeSource.Contains($ForbiddenVscodeReadmeWording)) {
            throw "VS Code README should describe quick fixes as edits or repairs, not '$ForbiddenVscodeReadmeWording'"
        }
    }
    $ExtensionSource = Get-Content -LiteralPath $ExtensionJsPath -Raw
    $ArtifactOpenersSource = Get-Content -LiteralPath $ArtifactOpenersPath -Raw
    $CommandHandlersSource = Get-Content -LiteralPath $CommandHandlersPath -Raw
    $DecorationsSource = Get-Content -LiteralPath $DecorationsPath -Raw
    $CompletionProviderSource = Get-Content -LiteralPath $CompletionProviderPath -Raw
    $DiagnosticsProviderSource = Get-Content -LiteralPath $DiagnosticsProviderPath -Raw
    $HoverProviderSource = Get-Content -LiteralPath $HoverProviderPath -Raw
    $CodeActionProviderSource = Get-Content -LiteralPath $CodeActionProviderPath -Raw
    $FoldingRangeProviderSource = Get-Content -LiteralPath $FoldingRangeProviderPath -Raw
    $FormattingProviderSource = Get-Content -LiteralPath $FormattingProviderPath -Raw
    $NavigationProvidersSource = Get-Content -LiteralPath $NavigationProvidersPath -Raw
    $SemanticTokensProviderSource = Get-Content -LiteralPath $SemanticTokensProviderPath -Raw
    $LocalCodeActionsSource = Get-Content -LiteralPath $LocalCodeActionsPath -Raw
    $LspCodeActionsSource = Get-Content -LiteralPath $LspCodeActionsPath -Raw
    $LspKindsSource = Get-Content -LiteralPath $LspKindsPath -Raw
    $LspNavigationSource = Get-Content -LiteralPath $LspNavigationPath -Raw
    $LspRangesSource = Get-Content -LiteralPath $LspRangesPath -Raw
    $LspRequestsSource = Get-Content -LiteralPath $LspRequestsPath -Raw
    $LspSemanticTokensSource = Get-Content -LiteralPath $LspSemanticTokensPath -Raw
    $ArtifactRegistrySource = Get-Content -LiteralPath $ArtifactRegistryPath -Raw
    $EditorMetadataLoaderSource = Get-Content -LiteralPath $EditorMetadataLoaderPath -Raw
    $ExecutionProfilesSource = Get-Content -LiteralPath $ExecutionProfilesPath -Raw
    $ModuleStatusSource = Get-Content -LiteralPath $ModuleStatusPath -Raw
    $RuntimeDiscoverySource = Get-Content -LiteralPath $RuntimeDiscoveryPath -Raw
    $ReviewPanelRendererSource = Get-Content -LiteralPath $ReviewPanelRendererPath -Raw
    $DiagnosticsSource = $ExtensionSource + "`n" + $DiagnosticsProviderSource
    foreach ($RequiredStatusBarToken in @(
        "createStatusBarItem",
        'diagnosticsStatusBar.name = "EngLang Problems"',
        'diagnosticsStatusBar.command = "englang.showToolingStatus"',
        "function updateDiagnosticsStatusBar",
        "function updateDiagnosticsStatusBarForDocument",
        "diagnosticSeverityCounts",
        "diagnosticsStatusBarCountText",
        "diagnosticsStatusBarUpdateState",
        "vscode.languages.onDidChangeDiagnostics",
        "updateDiagnosticsStatusBarForDocument(event.document)",
        "updateDiagnosticsStatusBar(editor?.document)",
        'EngLang Problems: ${countText}',
        "Click to open EngLang: Show Tooling Status.",
        "run EngLang: Refresh Problems for an unsaved-buffer check",
        "File diagnostics use the saved file; save before refreshing"
    )) {
        if (-not $ExtensionSource.Contains($RequiredStatusBarToken)) {
            throw "VS Code extension missing EngLang Problems status bar token $RequiredStatusBarToken"
        }
    }
    foreach ($ForbiddenCommandWording in @(
        "Current File Review JSON",
        "Last Run Review JSON",
        "Last Run Output Manifest",
        "Last Run External Process Results",
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
        if ($PackageSource.Contains($ForbiddenCommandWording) -or $ExtensionSource.Contains($ForbiddenCommandWording) -or $ArtifactOpenersSource.Contains($ForbiddenCommandWording) -or $ArtifactRegistrySource.Contains($ForbiddenCommandWording)) {
            throw "VS Code command wording should use user-facing artifact names instead of '$ForbiddenCommandWording'"
        }
    }
    if (-not $ReviewPanelRendererSource.Contains('require("./moduleStatus")')) {
        throw "VS Code workflow module panel must load wording helpers from moduleStatus.js"
    }
    foreach ($RequiredModuleWordingToken in @("moduleStatusDisplay", "moduleStatusDetailDisplay", "moduleBackingLabel", "Compiler/runtime", "No executable backing")) {
        if (-not ($ExtensionSource.Contains($RequiredModuleWordingToken) -or $ReviewPanelRendererSource.Contains($RequiredModuleWordingToken) -or $ModuleStatusSource.Contains($RequiredModuleWordingToken))) {
            throw "VS Code workflow module panel missing wording token $RequiredModuleWordingToken"
        }
    }
    if ($ModuleStatusSource.Contains('return "Native workflow support";')) {
        throw "VS Code workflow module fallback label should stay short; use compiler status_label for full registry wording"
    }
    if ($ExtensionSource.Contains("function moduleStatusDisplay") -or $ExtensionSource.Contains("function moduleBackingLabel")) {
        throw "VS Code extension must keep workflow module wording helpers in moduleStatus.js"
    }
    $RuntimeDiscoverySourceCombined = $ExtensionSource + "`n" + $RuntimeDiscoverySource
    foreach ($RequiredRuntimeDiscoveryToken in @(
        'require("./runtimeDiscovery")',
        "workspaceRoot(document)",
        "currentWorkspaceRoot",
        "engConfig(document)",
        "findRuntime(context, document)",
        "findLspRuntime(context, document)",
        "findLspRuntimeForRoot(context, root, document)",
        'englang", uri',
        '"runtimePath"',
        '"lspPath"',
        'path.join(context.extensionPath, "bin", "eng.exe")',
        'path.join(context.extensionPath, "bin", "eng-lsp.exe")'
    )) {
        if (-not $RuntimeDiscoverySourceCombined.Contains($RequiredRuntimeDiscoveryToken)) {
            throw "VS Code extension missing runtime discovery token $RequiredRuntimeDiscoveryToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./runtimeDiscovery")') -or -not $RuntimeDiscoverySource.Contains("findRuntime") -or -not $RuntimeDiscoverySource.Contains("findLspRuntimeForRoot")) {
        throw "VS Code extension must load runtime discovery helpers from runtimeDiscovery.js"
    }
    if ($ExtensionSource.Contains("function findRuntime") -or $ExtensionSource.Contains("function findLspRuntime") -or $ExtensionSource.Contains("function findLspRuntimeForRoot") -or $ExtensionSource.Contains("function workspaceRoot") -or $ExtensionSource.Contains("function currentWorkspaceRoot") -or $ExtensionSource.Contains("function engConfig")) {
        throw "VS Code extension must keep runtime discovery helpers in runtimeDiscovery.js"
    }
    $CommandSourceCombined = $ExtensionSource + "`n" + $CommandHandlersSource
    $RegisteredCommands = @([regex]::Matches($ExtensionSource, 'registerCommand\("([^"]+)"') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
    foreach ($Command in $Commands) {
        if ($RegisteredCommands -notcontains $Command) {
            throw "VS Code package command $Command is not registered in extension.js"
        }
    }
    $AllowedCodeOnlyCommands = @("englang.switchProblemsSource")
    foreach ($Command in $AllowedCodeOnlyCommands) {
        if ($RegisteredCommands -notcontains $Command) {
            throw "VS Code extension must keep $Command as a code-only compatibility alias"
        }
    }
    $UnexpectedRegisteredOnlyCommands = @($RegisteredCommands | Where-Object {
        $Commands -notcontains $_ -and $AllowedCodeOnlyCommands -notcontains $_
    })
    if ($UnexpectedRegisteredOnlyCommands.Count -gt 0) {
        throw "VS Code extension registers command(s) not exposed in package metadata: $($UnexpectedRegisteredOnlyCommands -join ', ')"
    }
    if (-not $ExtensionSource.Contains('require("./commandHandlers")') -or -not $CommandHandlersSource.Contains("function createCommandHandlers")) {
        throw "VS Code extension must load command handlers from commandHandlers.js"
    }
    $ReviewPanelSourceCombined = $ExtensionSource + "`n" + $CommandHandlersSource + "`n" + $ReviewPanelRendererSource + "`n" + $DecorationsSource
    if ($ReviewPanelSourceCombined.Contains('reviewValue(module, "backing")')) {
        throw "VS Code workflow module panel must not display raw registry backing keys"
    }
    if ($ExtensionSource.Contains("--entry") -or $ExtensionSource.Contains("runEntry")) {
        throw "VS Code extension run command must use top-level execution without entry flags"
    }
    if (-not $CommandHandlersSource.Contains("--save-artifacts")) {
        throw "VS Code extension run command must save artifacts for review/open-artifact commands"
    }
    if (-not $CommandHandlersSource.Contains("--profile") -or -not $CommandHandlersSource.Contains("executionProfile") -or -not $CommandHandlersSource.Contains("switchExecutionProfile")) {
        throw "VS Code extension run command must expose and pass an execution profile"
    }
    foreach ($RequiredToolingStatusToken in @(
        "async function showToolingStatus",
        "function toolingStatusPayload",
        "const currentFileHighlightProbe = await toolingStatusHighlightProbe(context)",
        "const currentFileProblemsProbe = toolingStatusProblemsProbe()",
        "const nativeWorkflowProbe = toolingStatusNativeWorkflowProbe(document)",
        "current_file_highlights: currentFileHighlightProbe?.summary",
        "current_file_problems: currentFileProblemsProbe?.summary",
        "current_file_probe: currentFileHighlightProbe",
        "current_file_probe: currentFileProblemsProbe",
        'cursor: "EngLang: Inspect Problem at Cursor"',
        'copy_cursor: "EngLang: Copy Problem at Cursor"',
        'inspect_problem_at_cursor: "EngLang: Inspect Problem at Cursor"',
        'copy_problem_at_cursor: "EngLang: Copy Problem at Cursor"',
        "function toolingStatusProblemsProbe()",
        "function toolingStatusNativeWorkflowProbe(document)",
        "native_workflows: nativeWorkflowProbe",
        "latest_process_artifact",
        "latest_run_graph_artifacts",
        "full_evidence_gate",
        "diagnosticsCollection.get(document.uri)",
        "function toolingStatusProblemRow(document, diagnostic, index)",
        "diagnostic_range_status",
        "function toolingStatusProblemRangeText(range)",
        "async function toolingStatusHighlightProbe(context)",
        'typeof lspRequests?.snapshotDocumentSource !== "function"',
        "Current-file highlight probing is unavailable because live editor checks are not configured.",
        "function toolingHighlightProbeSummary(tokenCount, rangeOverlapCount)",
        "range_overlap_status: highlightRangeOverlapStatus(tokenRows.length, rangeOverlaps.length)",
        "Current file returned no role-aware highlight tokens.",
        "with no overlapping ranges.",
        "function executableStatus",
        "findLspRuntime",
        "editor_client",
        "request_model",
        "long_running_language_server",
        "features",
        "summary:",
        "diagnosticsModeChangeSummary",
        "diagnosticsStatusSummary",
        "toolStatusSummary",
        "toolStatusSummary(liveEditorTool, `"live editor checks`")",
        "const problemsSource = mode",
        "updates_while_typing",
        "source_label",
        "diagnosticsProblemsSource",
        "source_label: diagnosticsProblemsSource(mode)",
        "const sourceLabel = diagnosticsProblemsSource(mode)",
        'with source ${sourceLabel}',
        'use source ${sourceLabel}',
        "configured_path_status",
        "availability",
        "role_aware_colors",
        "role_aware_highlighting",
        "highlighting_model",
        "highlightingStatusSummary",
        "semanticScopeMapStatus",
        "fallback_scope_map",
        "inspection_commands",
        "missing_token_types",
        "missing_modifiers",
        "diagnostics_mode",
        "saved_file_diagnostics_on_open_save",
        "live_typing_diagnostics_enabled",
        "semantic_highlighting",
        "review_risk_decorations"
    )) {
        if (-not $CommandHandlersSource.Contains($RequiredToolingStatusToken)) {
            throw "VS Code command handlers missing tooling status token $RequiredToolingStatusToken"
        }
    }
    foreach ($RequiredToolingStatusAlias in @(
        "const checkAndRunTool = executableStatus",
        "const liveEditorTool = executableStatus",
        "tools:",
        "check_and_run: checkAndRunTool",
        "live_editor: liveEditorTool",
        "eng: checkAndRunTool",
        "eng_lsp: liveEditorTool",
        'request_model: "on-demand live editor checks"',
        "long_running_language_server: false",
        'live_buffer_tool: "live_editor"',
        'file_check_tool: "check_and_run"'
    )) {
        if (-not $CommandHandlersSource.Contains($RequiredToolingStatusAlias)) {
            throw "VS Code tooling status must expose user-facing tool aliases while keeping executable compatibility keys"
        }
    }
    foreach ($ForbiddenToolingStatusWording in @(
        "fresh live editor request",
        "short-lived live editor request",
        "live editor requests"
    )) {
        if ($CommandHandlersSource.Contains($ForbiddenToolingStatusWording)) {
            throw "VS Code tooling status must describe editor checks in user-facing wording: $ForbiddenToolingStatusWording"
        }
    }
    foreach ($RequiredProblemCursorInspectorToken in @(
        "async function showProblemAtCursor()",
        "async function copyProblemAtCursor()",
        "function activeEngEditorOrWarn()",
        "function problemCursorPayload(document, cursor)",
        "vscode.env.clipboard.writeText(JSON.stringify(copyReady, null, 2))",
        'showInformationMessage("EngLang problem copied to clipboard.")',
        "diagnosticsCollection.get(document.uri)",
        "matching_problems: matchingProblems",
        "nearest_problems: nearestProblems",
        "line_problems: lineProblems",
        "diagnostic_source_text: toolingStatusProblemSourceText(document, range)",
        "source_line_text: toolingStatusProblemLineText(document, range)",
        "function toolingStatusProblemSourceText(document, range)",
        "function toolingStatusProblemLineText(document, range)",
        "function toolingStatusProblemTruncatedText(text)",
        "text: problem.diagnostic_source_text",
        "line_text: problem.source_line_text",
        "function cursorProblemStatus(matchingProblems, nearestProblems, fileProblemCount, diagnosticsAvailable = true)",
        "function problemCopyReady(problem)"
    )) {
        if (-not $CommandHandlersSource.Contains($RequiredProblemCursorInspectorToken)) {
            throw "VS Code Problems cursor inspector missing contract token $RequiredProblemCursorInspectorToken"
        }
    }
    if (-not $CommandHandlersSource.Contains('require("./executionProfiles")') -or -not $ExecutionProfilesSource.Contains("EXECUTION_PROFILES") -or -not $ExecutionProfilesSource.Contains('"normal"') -or -not $ExecutionProfilesSource.Contains('"safe"') -or -not $ExecutionProfilesSource.Contains('"repro"')) {
        throw "VS Code extension must load user-facing execution profiles from executionProfiles.js"
    }
    if ($ExtensionSource.Contains("const EXECUTION_PROFILES = [") -or $CommandHandlersSource.Contains("const EXECUTION_PROFILES = [")) {
        throw "VS Code extension must keep execution profile labels in executionProfiles.js"
    }
    if (-not $CommandHandlersSource.Contains("runExample") -or -not $CommandHandlersSource.Contains("findExampleFiles") -or -not $CommandHandlersSource.Contains('"official"') -or -not $CommandHandlersSource.Contains('"workflows"')) {
        throw "VS Code extension must expose an example runner for official and workflow examples"
    }
    if (-not $CommandHandlersSource.Contains('"review", document.uri.fsPath, "--json"')) {
        throw "VS Code extension must expose a current-file review JSON command"
    }
    if (-not $CommandHandlersSource.Contains('require("./reviewPanelRenderer")') -or -not $ReviewPanelRendererSource.Contains("function renderReviewSummaryHtml") -or -not $ReviewPanelRendererSource.Contains("function reviewPanelArtifacts")) {
        throw "VS Code extension must load review panel rendering helpers from reviewPanelRenderer.js"
    }
    if (-not $ReviewPanelRendererSource.Contains("Source / Status") -or -not $ReviewPanelRendererSource.Contains("response_source") -or -not $ReviewPanelRendererSource.Contains("boundarySourceOrStatusCell")) {
        throw "VS Code review panel must label network response source separately from generic status"
    }
    if ($ExtensionSource.Contains("function renderReviewSummaryHtml") -or $ExtensionSource.Contains("function renderReviewTable") -or $ExtensionSource.Contains("function lineValue(item)") -or $ExtensionSource.Contains("function reviewPanelArtifacts")) {
        throw "VS Code extension must keep review panel rendering helpers in reviewPanelRenderer.js"
    }
    if (-not $CommandSourceCombined.Contains("openReviewPanel") -or -not $CommandHandlersSource.Contains("createWebviewPanel") -or -not $ReviewPanelSourceCombined.Contains("renderReviewSummaryHtml")) {
        throw "VS Code extension must expose a current-file review summary panel"
    }
    if (-not $ReviewPanelSourceCombined.Contains("<h2>Inputs</h2>") -or -not $ReviewPanelSourceCombined.Contains("<h2>Schemas</h2>") -or -not $ReviewPanelSourceCombined.Contains("<h2>Units And Quantities</h2>") -or -not $ReviewPanelSourceCombined.Contains("<h2>Derived Values</h2>") -or -not $ReviewPanelSourceCombined.Contains("<h2>Caches</h2>")) {
        throw "VS Code extension review panel must expose core ReviewDocument sections"
    }
    if (-not $ReviewPanelSourceCombined.Contains("<h2>Review Fingerprint</h2>")) {
        throw "VS Code extension review panel must label semantic_hash as Review Fingerprint"
    }
    if ($ReviewPanelSourceCombined.Contains("<h2>Semantic Hash</h2>")) {
        throw "VS Code extension review panel must not expose internal Semantic Hash wording"
    }
    if (-not $CommandHandlersSource.Contains("onDidReceiveMessage") -or -not $ReviewPanelSourceCombined.Contains("data-source-line") -or -not $CommandHandlersSource.Contains("openSourceLine")) {
        throw "VS Code extension review panel must support source-line navigation"
    }
    foreach ($RequiredReviewPanelSourceColumnToken in @(
        "function columnValue(item)",
        "data-source-column",
        "source_column",
        "sourceColumn",
        "message.column",
        "sourceColumnCharacter",
        "characterBytes",
        "byteOffset + characterBytes > targetByte"
    )) {
        if (-not $ReviewPanelSourceCombined.Contains($RequiredReviewPanelSourceColumnToken) -and -not $CommandHandlersSource.Contains($RequiredReviewPanelSourceColumnToken)) {
            throw "VS Code extension review panel missing source-column navigation token $RequiredReviewPanelSourceColumnToken"
        }
    }
    foreach ($RequiredSourceLineToken in @(
        "function lineValue(item)",
        "source_span",
        "sourceSpan",
        "source_line",
        "sourceLine",
        "reviewRiskLineNumber"
    )) {
        if (-not $ReviewPanelSourceCombined.Contains($RequiredSourceLineToken)) {
            throw "VS Code extension review panel missing normalized source-line token $RequiredSourceLineToken"
        }
    }
    foreach ($RequiredDiagnosticsSourceColumnToken in @(
        "function diagnosticRange(document, item)",
        "function sourceColumnNumber(item)",
        "source_span?.column",
        "sourceColumnCharacter(lineText, sourceColumn)",
        "Buffer.byteLength(character, `"utf8`")",
        "diagnosticTokenEndCharacter",
        "diagnosticFallbackRangeForCode(lineText, item, sourceColumn)",
        "const optionNames = diagnosticOptionNames(code)",
        "const optionRange = optionValueRange(lineText, optionNames, searchStart)",
        "function diagnosticOptionNames(code)",
        "function optionValueRange(lineText, optionNames, startCharacter = 0)",
        "function optionValueRangeFrom(lineText, optionName, startCharacter = 0)",
        "stripLineComment(lineText)",
        "E-NET-RETRY-POLICY",
        "E-PROCESS-RETRY-POLICY",
        "E-NET-BODY-SIZE-LIMIT",
        "E-SAMPLING-COUNT-INVALID",
        "E-ML-ARGS-002",
        "E-CACHE-KEY-NONDETERMINISTIC",
        "E-SOLVE-MAX-ITER-INVALID",
        "variable_scale",
        "algebraic_initialization",
        "firstNeedleRange(lineText, [`":=`"], searchStart)",
        "firstNeedleRange(lineText, [`"==`"], searchStart)",
        "firstNeedleRange(lineText, [`"struct Args`", `"struct`"], 0)",
        "firstNeedleRange(lineText, [`"script`"], searchStart)",
        "optionKeyRange(lineText, `"fixture`")",
        "memberFieldRange(lineText, `"hash`", searchStart)",
        "functionCallNameRange(lineText, `"sum`", searchStart)",
        "logLevelRange(lineText)",
        "netUrlLiteralRange(lineText, searchStart)",
        "diagnosticBacktickRange(lineText, item)"
    )) {
        if (-not $DiagnosticsProviderSource.Contains($RequiredDiagnosticsSourceColumnToken)) {
            throw "VS Code diagnostics provider missing source-column range token $RequiredDiagnosticsSourceColumnToken"
        }
    }
    if (-not $ReviewPanelSourceCombined.Contains("reviewPanelArtifacts") -or -not $ReviewPanelSourceCombined.Contains("data-artifact-id") -or -not $CommandHandlersSource.Contains("openArtifact")) {
        throw "VS Code extension review panel must expose clickable last-run artifacts"
    }
    foreach ($ForbiddenCommandHandlerEntrypointToken in @(
        "async function runActiveFile",
        "async function runExample",
        "async function switchExecutionProfile",
        "async function switchDiagnosticsMode",
        "async function showToolingStatus",
        "async function showProblemAtCursor",
        "async function copyProblemAtCursor",
        "async function reviewActiveFile",
        "async function openReviewPanel",
        "async function showSemanticTokensDebug",
        "async function showSemanticTokenAtCursor",
        "async function copySemanticTokenAtCursor",
        "function runReviewForDocument",
        "function findExampleFiles",
        "function executionProfile",
        "function toolingStatusPayload",
        "function executableStatus"
    )) {
        if ($ExtensionSource.Contains($ForbiddenCommandHandlerEntrypointToken)) {
            throw "VS Code extension must keep command handlers in commandHandlers.js"
        }
    }
    $ArtifactOpenersCombined = $ExtensionSource + "`n" + $ArtifactOpenersSource
    if (-not $ExtensionSource.Contains('require("./artifactOpeners")') -or -not $ArtifactOpenersSource.Contains("function createArtifactOpeners") -or -not $ArtifactOpenersSource.Contains("function outputManifestArtifactItems")) {
        throw "VS Code extension must load artifact-opening helpers from artifactOpeners.js"
    }
    if ($ExtensionSource.Contains("function openLastRunArtifact") -or $ExtensionSource.Contains("function openGeneratedOutputArtifactPicker") -or $ExtensionSource.Contains("function outputManifestArtifactItems")) {
        throw "VS Code extension must keep artifact-opening helpers in artifactOpeners.js"
    }
    foreach ($ForbiddenArtifactOpenerWording in @("No build/result/output_manifest.json", "last output_manifest.json")) {
        if ($ArtifactOpenersCombined.Contains($ForbiddenArtifactOpenerWording)) {
            throw "VS Code artifact opener should use user-facing generated output wording instead of '$ForbiddenArtifactOpenerWording'"
        }
    }
    $OpenLastRunPickerIndex = $ArtifactOpenersSource.IndexOf("async function openLastRunArtifactPicker()")
    $OpenLastRunQuickPickIndex = $ArtifactOpenersSource.IndexOf("lastRunArtifactQuickPickItems(root)", [Math]::Max($OpenLastRunPickerIndex, 0))
    if ($OpenLastRunPickerIndex -lt 0 -or $OpenLastRunQuickPickIndex -lt $OpenLastRunPickerIndex) {
        throw "VS Code artifact opener must expose the last-run artifact picker"
    }
    $OpenLastRunPickerPreamble = $ArtifactOpenersSource.Substring($OpenLastRunPickerIndex, $OpenLastRunQuickPickIndex - $OpenLastRunPickerIndex)
    if (-not $OpenLastRunPickerPreamble.Contains("if (!root)") -or -not $OpenLastRunPickerPreamble.Contains('vscode.window.showWarningMessage("Open an EngLang workspace folder first.");') -or -not $OpenLastRunPickerPreamble.Contains("return;")) {
        throw "VS Code last-run artifact picker must require an EngLang workspace before showing choices"
    }
    if (-not $ArtifactOpenersSource.Contains('require("./artifactRegistry")') -or -not $ArtifactRegistrySource.Contains("LAST_RUN_ARTIFACTS") -or -not $ArtifactRegistrySource.Contains("Report HTML") -or -not $ArtifactRegistrySource.Contains("Generated Output List")) {
        throw "VS Code extension must load user-facing artifact labels from artifactRegistry.js"
    }
    if (-not $ArtifactRegistrySource.Contains("function lastRunArtifactDisplay") -or -not $ArtifactRegistrySource.Contains("0 external processes") -or -not $ArtifactOpenersSource.Contains("lastRunArtifactDisplay") -or -not $ReviewPanelRendererSource.Contains("lastRunArtifactDisplay")) {
        throw "VS Code artifact picker and review panel must share zero-process process-results wording"
    }
    if (-not $ArtifactRegistrySource.Contains("function lastRunArtifactAvailability") -or -not $ArtifactOpenersSource.Contains("lastRunArtifactAvailability") -or -not $ReviewPanelRendererSource.Contains("lastRunArtifactAvailability") -or -not $ArtifactOpenersSource.Contains("available: availability.exists")) {
        throw "VS Code artifact picker and review panel must share last-run artifact availability state"
    }
    $LastRunArtifactQuickPickIndex = $ArtifactOpenersSource.IndexOf("function lastRunArtifactQuickPickItems(root)")
    $OutputManifestArtifactItemsIndex = $ArtifactOpenersSource.IndexOf("function outputManifestArtifactItems", [Math]::Max($LastRunArtifactQuickPickIndex, 0))
    if ($LastRunArtifactQuickPickIndex -lt 0 -or $OutputManifestArtifactItemsIndex -lt $LastRunArtifactQuickPickIndex -or -not $ArtifactOpenersSource.Contains("LAST_RUN_ARTIFACTS.map((artifact)")) {
        throw "VS Code last-run artifact picker must be built directly from the artifact registry"
    }
    $LastRunArtifactQuickPickSource = $ArtifactOpenersSource.Substring($LastRunArtifactQuickPickIndex, $OutputManifestArtifactItemsIndex - $LastRunArtifactQuickPickIndex)
    if ($LastRunArtifactQuickPickSource.Contains(".sort((left, right)") -or $LastRunArtifactQuickPickSource.Contains("left.label.localeCompare")) {
        throw "VS Code last-run artifact picker must keep registry workflow order instead of resorting artifacts"
    }
    if (-not $ExtensionSource.Contains("onDidChangeTextDocument") -or -not $DiagnosticsSource.Contains("--snapshot-stdin")) {
        throw "VS Code extension must support debounced unsaved-buffer diagnostics through eng-lsp --snapshot-stdin"
    }
    foreach ($RequiredEditorChangeCacheToken in @(
        "function clearCachedEditorSnapshot(document)",
        "clearCachedEditorSnapshot(event.document)",
        "reviewCache.delete(document.uri.fsPath)",
        "updateReviewRiskDecorations(document, undefined)",
        "updateSemanticSymbolDecorations(document, undefined)"
    )) {
        if (-not $ExtensionSource.Contains($RequiredEditorChangeCacheToken)) {
            throw "VS Code extension must clear stale cached editor review/highlight state on buffer edits: $RequiredEditorChangeCacheToken"
        }
    }
    if (-not $DiagnosticsProviderSource.Contains('this.diagnosticsRuntime?.(document) !== "lsp-snapshot"')) {
        throw "VS Code live buffer diagnostics must only run when diagnostics mode is live"
    }
    foreach ($RequiredManualRefreshModeToken in @(
        'const runtimeMode = this.diagnosticsRuntime?.(document);',
        'runtimeMode === "lsp-snapshot"',
        'file mode uses saved-file checks; save the file to refresh Problems',
        'EngLang file diagnostics use saved files. Save the file, or switch Diagnostics Mode to live for unsaved Problems.'
    )) {
        if (-not $DiagnosticsProviderSource.Contains($RequiredManualRefreshModeToken)) {
            throw "VS Code manual Problems refresh must respect diagnostics mode for dirty buffers: $RequiredManualRefreshModeToken"
        }
    }
    $LspRequestSourceCombined = $ExtensionSource + "`n" + $LspRequestsSource
    if (-not $ExtensionSource.Contains('require("./lspRequests")') -or -not $LspRequestsSource.Contains("function createLspRequests")) {
        throw "VS Code extension must load LSP request helpers from lspRequests.js"
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
        if (-not $LspRequestsSource.Contains($RequiredSnapshotReuseToken)) {
            throw "VS Code extension missing shared LSP snapshot reuse token $RequiredSnapshotReuseToken"
        }
    }
    if (-not $ExtensionSource.Contains("clearSnapshotCache: lspRequests.clearSnapshotCache") -or -not $DiagnosticsProviderSource.Contains("this.clearSnapshotCache(document)") -or -not $LspRequestsSource.Contains("function clearSnapshotCache(document)")) {
        throw "VS Code extension must clear shared LSP snapshot cache on document changes and close"
    }
    $SnapshotDocumentSourceIndex = $LspRequestsSource.IndexOf("function snapshotDocumentSource")
    if ($SnapshotDocumentSourceIndex -lt 0) {
        throw "VS Code extension missing shared snapshot document source helper"
    }
    $SnapshotDocumentSource = $LspRequestsSource.Substring($SnapshotDocumentSourceIndex, $LspRequestsSource.IndexOf("function snapshotCacheKey") - $SnapshotDocumentSourceIndex)
    foreach ($RequiredSnapshotFreshnessToken in @(
        "const documentVersion = document.version;",
        "const documentText = document.getText();",
        "document.version !== documentVersion",
        "child.stdin.end(documentText)"
    )) {
        if (-not $SnapshotDocumentSource.Contains($RequiredSnapshotFreshnessToken)) {
            throw "VS Code snapshot live editor requests must guard stale document versions with $RequiredSnapshotFreshnessToken"
        }
    }
    $StdinJsonRequestIndex = $LspRequestsSource.IndexOf("function stdinJsonRequest")
    if ($StdinJsonRequestIndex -lt 0) {
        throw "VS Code extension missing shared stdin JSON request helper"
    }
    $StdinJsonRequestSource = $LspRequestsSource.Substring($StdinJsonRequestIndex)
    foreach ($RequiredStdinRequestFreshnessToken in @(
        "const documentVersion = document.version;",
        "const documentText = document.getText();",
        "document.version !== documentVersion",
        "child.stdin.end(documentText)"
    )) {
        if (-not $StdinJsonRequestSource.Contains($RequiredStdinRequestFreshnessToken)) {
            throw "VS Code stdin live editor requests must guard stale document versions with $RequiredStdinRequestFreshnessToken"
        }
    }
    $ProviderFreshnessContracts = @(
        @{ Label = "completion"; Source = $CompletionProviderSource; MinimumVersionGuardCount = 1 },
        @{ Label = "hover"; Source = $HoverProviderSource; MinimumVersionGuardCount = 1 },
        @{ Label = "navigation"; Source = $NavigationProvidersSource; MinimumVersionGuardCount = 2 },
        @{ Label = "folding"; Source = $FoldingRangeProviderSource; MinimumVersionGuardCount = 1 },
        @{ Label = "semantic tokens"; Source = $SemanticTokensProviderSource; MinimumVersionGuardCount = 1 },
        @{ Label = "formatting"; Source = $FormattingProviderSource; MinimumVersionGuardCount = 2 },
        @{ Label = "code action"; Source = $CodeActionProviderSource; MinimumVersionGuardCount = 1 }
    )
    foreach ($ProviderFreshnessContract in $ProviderFreshnessContracts) {
        $VersionGuardCount = [regex]::Matches($ProviderFreshnessContract.Source, "const documentVersion = document\.version;").Count
        if ($VersionGuardCount -lt $ProviderFreshnessContract.MinimumVersionGuardCount -or -not $ProviderFreshnessContract.Source.Contains("document.version !== documentVersion")) {
            throw "VS Code $($ProviderFreshnessContract.Label) provider must guard stale document versions before using live or cached editor results"
        }
    }
    foreach ($RequiredLiveEditorOutputToken in @(
        "Live editor check failed:",
        "Unable to parse EngLang live editor data:",
        "Completion lookup failed",
        "Unable to parse EngLang completion data",
        "Definition lookup failed",
        "Unable to parse EngLang definition data"
    )) {
        if (-not $LspRequestsSource.Contains($RequiredLiveEditorOutputToken)) {
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
        if ($LspRequestSourceCombined.Contains($ForbiddenLiveEditorOutputToken)) {
            throw "VS Code extension output must use live editor wording, not $ForbiddenLiveEditorOutputToken"
        }
    }
    foreach ($ForbiddenLspRequestEntrypointToken in @(
        "function snapshotDocumentSource",
        "function workspaceSymbolsForQuery",
        "function workspaceSymbolsForFolder",
        "function completionSnapshotForPosition",
        "function definitionSnapshotForPosition",
        "function formatDocumentSource",
        "function codeActionsForDocumentSource",
        "function snapshotCacheKey"
    )) {
        if ($ExtensionSource.Contains($ForbiddenLspRequestEntrypointToken)) {
            throw "VS Code extension must keep LSP subprocess request helpers in lspRequests.js"
        }
    }
    $HoverSource = $ExtensionSource + "`n" + $HoverProviderSource
    foreach ($RequiredHoverToken in @(
        'require("./hoverProvider")',
        "new EngHoverProvider(context",
        "async provideHover",
        "findHoverForWord",
        "hoverNameMatches",
        "hoverFromSnapshot",
        "hoverMarkdown",
        "hoverKindLabel",
        "hoverStatusLabel",
        "HOVER_KIND_LABELS",
        'coverage_result_field: "Coverage result field"',
        'table_field: "Table field"',
        'model_field: "Model field"',
        'db_connection_field: "DB connection field"',
        'case_result_collection_table_field: "Case result collection field"',
        "hoverDisplayUnit",
        "snapshotDocumentSource",
        "cachedSnapshotForDocument",
        "cacheSnapshotForDocument",
        "hover.status",
        "hover.kind"
    )) {
        if (-not $HoverSource.Contains($RequiredHoverToken)) {
            throw "VS Code extension missing live hover snapshot token $RequiredHoverToken"
        }
    }
    if ($HoverProviderSource.Contains('Kind: \`${hover.kind}\`') -or $HoverProviderSource.Contains('Kind: `${hover.kind}`')) {
        throw "VS Code hover must show user-facing kind labels instead of raw payload ids"
    }
    if ($HoverProviderSource.Contains('Status: \`${hover.status}\`')) {
        throw "VS Code hover must show user-facing status labels instead of raw payload ids"
    }
    if ($ExtensionSource.Contains("class EngHoverProvider") -or $ExtensionSource.Contains("function findHoverForWord") -or $ExtensionSource.Contains("function hoverNameMatches") -or $ExtensionSource.Contains("function hoverRangeAtPosition") -or $ExtensionSource.Contains("function hoverCandidatesAtPosition")) {
        throw "VS Code extension must keep hover provider helpers in hoverProvider.js"
    }
    if (-not $ExtensionSource.Contains('require("./editorMetadata")') -or -not $ExtensionSource.Contains("loadEditorMetadata(__dirname)")) {
        throw "VS Code extension must load editor metadata through editorMetadata.js"
    }
    if (-not $EditorMetadataLoaderSource.Contains("englang-editor-metadata.json") -or -not $EditorMetadataLoaderSource.Contains("semantic_token_legend") -or -not $EditorMetadataLoaderSource.Contains("completion_items") -or -not $EditorMetadataLoaderSource.Contains("syntax_catalog") -or -not $EditorMetadataLoaderSource.Contains("constants") -or -not $EditorMetadataLoaderSource.Contains("workflow_status_literals") -or -not $EditorMetadataLoaderSource.Contains("operator_words") -or -not $EditorMetadataLoaderSource.Contains("legacy_unit_aliases") -or -not $EditorMetadataLoaderSource.Contains("keyword_groups") -or -not $EditorMetadataLoaderSource.Contains("hyphenated_workflow_builtins") -or -not $EditorMetadataLoaderSource.Contains("legacy_workflow_builtin_aliases") -or -not $EditorMetadataLoaderSource.Contains("legacy_workflow_option_aliases") -or -not $EditorMetadataLoaderSource.Contains("public_types") -or -not $EditorMetadataLoaderSource.Contains("quantities") -or -not $EditorMetadataLoaderSource.Contains("units") -or -not $EditorMetadataLoaderSource.Contains("http_response_fields") -or -not $EditorMetadataLoaderSource.Contains("coverage_result_fields") -or -not $EditorMetadataLoaderSource.Contains("table_fields") -or -not $EditorMetadataLoaderSource.Contains("sample_table_fields") -or -not $EditorMetadataLoaderSource.Contains("db_connection_fields") -or -not $EditorMetadataLoaderSource.Contains("case_table_fields") -or -not $EditorMetadataLoaderSource.Contains("case_output_table_fields") -or -not $EditorMetadataLoaderSource.Contains("case_result_collection_table_fields") -or -not $EditorMetadataLoaderSource.Contains("model_fields") -or -not $EditorMetadataLoaderSource.Contains("prediction_table_fields")) {
        throw "VS Code editor metadata loader must read generated semantic legend, syntax catalog, workflow status literal, workflow builtin, legacy workflow aliases, public type, quantity, unit, HTTP response field, coverage result field, table field, sample table field, case table field, case result collection field, model field, prediction table field, and completion item metadata"
    }
    if ($EditorMetadataLoaderSource.Contains("metadata.completion_items ??") -or $EditorMetadataLoaderSource.Contains("completion_seed") -or -not $EditorMetadataLoaderSource.Contains("const completionItems = metadata.completion_items") -or -not $EditorMetadataLoaderSource.Contains("metadata.completion_items_count !== completionItems.length")) {
        throw "VS Code editor metadata loader must require completion_items as the only runtime completion catalog"
    }
    if ($ExtensionSource.Contains("const SEMANTIC_TOKEN_TYPES = [") -or $ExtensionSource.Contains("const SEMANTIC_TOKEN_MODIFIERS = [")) {
        throw "VS Code extension must not hardcode semantic token legend arrays"
    }
    $CompletionSource = $ExtensionSource + "`n" + $CompletionProviderSource + "`n" + $LspRequestsSource
    if (-not $ExtensionSource.Contains("COMPLETION_ITEMS") -or -not $CompletionProviderSource.Contains("completion.lsp_kind")) {
        throw "VS Code extension must use generated completion item metadata as the completion fallback"
    }
    foreach ($RequiredCompletionToken in @(
        'require("./completionProvider")',
        "EngCompletionProvider",
        "completionSnapshotForPosition",
        "cachedSnapshotForDocument",
        "completionItemsFromPayload",
        "argsFieldCompletionsFromDocument",
        "schemaBindingFieldCompletionsFromDocument",
        "workflowBindingFieldCompletionsFromDocument",
        "workflowBindingFieldCompletionsFromSource",
        "apply\s*\(",
        "schemaFieldsFromDocument",
        "promotedSchemaBindingsFromDocument",
        "fieldsForSchemaBinding",
        "fieldsForWorkflowBinding",
        "firstMappedFieldsForReceiver",
        "receiverLookupCandidates",
        "receiver.split(""."").filter(Boolean).pop()",
        "workflowBindingFields",
        "schema field",
        "argsFields",
        "isArgsReceiver",
        "args field",
        "httpResponseFields",
        "coverageResultFields",
        "tableFields",
        "promote\s+(?:csv|toml|json(?:\s+records)?)",
        "check\s+coverage",
        "isCoverageResultLikeReceiver",
        "isTableLikeReceiver",
        "sampleTableFields",
        "latin[_-]hypercube|grid|random|uniform",
        "caseTableFields",
        "caseOutputTableFields",
        "normalized.includes(""rendered"")",
        "normalized.includes(""blocked"")",
        "caseResultCollectionTableFields",
        "modelFields",
        "predictionTableFields",
        "train\s+regression",
        "predict\s+",
        "isCaseResultCollectionLikeReceiver",
        "isModelLikeReceiver",
        "isPredictionTableLikeReceiver",
        "httpResponseFieldCompletionsForContext",
        "localMemberCompletionsForContext",
        "memberAccessCompletionContext",
        "completionKindFromLsp",
        "new vscode.CompletionItem"
    )) {
        if (-not $CompletionSource.Contains($RequiredCompletionToken)) {
            throw "VS Code extension missing completion provider token $RequiredCompletionToken"
        }
    }
    foreach ($UniqueCompletionFunction in @(
        "argsFieldCompletionsFromDocument",
        "schemaBindingFieldCompletionsFromDocument",
        "workflowBindingFieldCompletionsFromDocument",
        "workflowBindingFieldCompletionsFromSource",
        "fieldsForWorkflowBinding",
        "firstMappedFieldsForReceiver",
        "receiverLookupCandidates",
        "httpResponseFieldCompletionsForContext",
        "localMemberCompletionsForContext",
        "isCaseResultCollectionLikeReceiver",
        "isModelLikeReceiver",
        "isPredictionTableLikeReceiver"
    )) {
        $FunctionDeclarationCount = [regex]::Matches($CompletionProviderSource, "function\s+$UniqueCompletionFunction\s*\(").Count
        if ($FunctionDeclarationCount -ne 1) {
            throw "VS Code completion provider must declare $UniqueCompletionFunction exactly once"
        }
    }

    if (-not $CompletionProviderSource.Contains("((?:[A-Za-z_][A-Za-z0-9_]*\.)*[A-Za-z_][A-Za-z0-9_]*)\.") -or -not $CompletionProviderSource.Contains("[receiver, lastSegment]")) {
        throw "VS Code member completions must preserve dotted receivers and fall back to the terminal receiver segment for generated field maps"
    }
    if (-not $CompletionProviderSource.Contains("function stripLineComment(text)") -or -not $CompletionProviderSource.Contains("function lineCommentStart(text)") -or -not $CompletionProviderSource.Contains("const withoutComment = stripLineComment(line)") -or $CompletionProviderSource.Contains("line.replace(/#.*/")) {
        throw "VS Code local field completions must strip # and // comments with string-aware parsing"
    }
    $SemanticProviderSource = $ExtensionSource + "`n" + $CommandHandlersSource + "`n" + $SemanticTokensProviderSource + "`n" + $LspSemanticTokensSource
    foreach ($RequiredSemanticDebugToken in @(
        "showSemanticTokensDebug",
        "showSemanticTokenAtCursor",
        "copySemanticTokenAtCursor",
        "semanticTokenCursorPayload(context, document, cursor)",
        "vscode.env.clipboard.writeText(JSON.stringify(copyReady, null, 2))",
        'showInformationMessage("EngLang highlight token copied to clipboard.")',
        "showHighlightUnavailableWarning",
        "highlight data unavailable:",
        "Show Tooling Status",
        "matching_tokens: matchingTokens",
        "nearest_tokens: nearestTokens",
        "nearest_token_count: nearestTokens.length",
        "cursor_token_hint: cursorTokenHint",
        "copy_ready: semanticTokenCopyReady(matchingTokens[0] ?? nearestTokens[0] ?? null)",
        "status: cursorHighlightStatus(matchingTokens, nearestTokens)",
        "function highlightInspectionStatus",
        "function highlightRangeOverlapStatus(tokenCount, rangeOverlapCount)",
        "function highlightDirectSelectorStatus",
        "tokensWithUnmappedSelectors > 0",
        "rangeOverlapCount > 0",
        "need direct selector mapping",
        "overlapping highlight range",
        "no overlapping ranges",
        "theme fallback scope coverage, direct selector mappings, and no overlapping ranges",
        "function cursorHighlightStatus",
        "semanticTokenCursorDistance",
        "cursor_distance: semanticTokenCursorDistance(token, cursor.character)",
        "character >= start && character < end",
        "line_tokens: lineTokens",
        "const rangeOverlaps = semanticTokenRangeOverlaps(document, tokenRows)",
        "range_overlap_count: rangeOverlaps.length",
        "range_overlaps: rangeOverlaps",
        "highlight_range_overlap_count: rangeOverlaps.length",
        "highlight_range_overlaps: rangeOverlaps",
        "const lineRangeOverlaps = semanticTokenRangeOverlaps(document, lineTokens)",
        "line_range_overlap_count: lineRangeOverlaps.length",
        "line_range_overlap_status: highlightRangeOverlapStatus(lineTokens.length, lineRangeOverlaps.length)",
        "line_range_overlaps: lineRangeOverlaps",
        "function semanticTokenRangeOverlaps(document, rows)",
        "function semanticTokenOverlapSide(row)",
        "range_text: semanticTokenRangeText(line, start + 1, end - start)",
        "semanticTokenDebugRow",
        "semanticTokenRangeText",
        "semanticTokenCopyReady",
        "function semanticTokenInspectorPanels",
        "const inspectorPanels = semanticTokenInspectorPanels(sample, semanticSelectors)",
        "inspector_panels: inspectorPanels",
        'panel_hint: inspectorPanels.length > 0 ? inspectorPanels.join(", ") : null',
        "inspector_panels: row.inspector_panels ?? []",
        "panel_hint: row.panel_hint ?? null",
        'if (modifiers.includes("workflowStep")) add("workflow")',
        'if (modifiers.includes("cache") || /cache|cache_key|cachekey|offline_response/.test(detailText)) add("network")',
        'tokenType === "namespace"',
        "range_text: rangeText",
        "copy_text: sample.text",
        "copy_range: rangeText",
        "copy_selector: primarySelector",
        "semanticTokenRange(document, token)?.contains(cursor)",
        "semanticTokenScopeMapFromPackage",
        "semantic_highlighting_enabled",
        "summary: {",
        "status: highlightInspectionStatus(tokenCount, tokensWithoutFallbackScope, tokensWithUnmappedSelectors, rangeOverlaps.length)",
        "fallback_scope_status: highlightFallbackStatus(tokenCount, tokensWithoutFallbackScope)",
        "direct_selector_status: highlightDirectSelectorStatus(tokenCount, tokensWithUnmappedSelectors)",
        "range_overlap_status: highlightRangeOverlapStatus(tokenCount, rangeOverlaps.length)",
        "token_count: tokenCount",
        "counts_by_type: tokenCounts",
        "counts_by_modifier: modifierCounts",
        "counts_by_selector: selectorCounts",
        "scope_map_entry_count: Object.keys(semanticTokenScopeMap).length",
        "scope_map_status: scopeMapStatus.status",
        "semantic_scope_map: scopeMapStatus",
        "tokens_without_fallback_scope: tokensWithoutFallbackScope",
        "tokens_with_unmapped_selectors: tokensWithUnmappedSelectors",
        "missing_scope_selectors: missingScopeSelectors",
        "unmapped_selector_counts: unmappedSelectorCounts",
        "const primarySelector = semanticSelectors[0] ?? sample.type",
        "primary_selector: primarySelector",
        'fallback_status: fallbackScopes.length > 0 ? "mapped" : "missing_fallback_scope"',
        'direct_selector_status: unmappedSelectors.length > 0 ? "missing_direct_scope" : "mapped"',
        "fallback_scope_count: fallbackScopes.length",
        "semantic_selectors: semanticSelectors",
        "unmapped_semantic_selectors: unmappedSelectors",
        "fallback_scopes: fallbackScopes",
        "legend: semanticTokens.legend ?? {}",
        "samples: {",
        "by_type: tokenSamplesByType",
        "by_modifier: tokenSamplesByModifier",
        "by_selector: tokenSamplesBySelector",
        "tokens: tokenRows",
        "raw: {",
        "highlight_count: tokenCount",
        "highlight_counts_by_category: tokenCounts",
        "highlight_counts_by_detail: modifierCounts",
        "highlight_counts_by_selector: selectorCounts",
        "highlight_samples_by_category: tokenSamplesByType",
        "highlight_samples_by_detail: tokenSamplesByModifier",
        "highlight_samples_by_selector: tokenSamplesBySelector",
        "token_counts_by_type: tokenCounts",
        "token_counts_by_modifier: modifierCounts",
        "token_counts_by_selector: selectorCounts",
        "token_samples_by_type: tokenSamplesByType",
        "token_samples_by_modifier: tokenSamplesByModifier",
        "token_samples_by_selector: tokenSamplesBySelector",
        "highlight_data: semanticTokens",
        "semantic_tokens: semanticTokens",
        'inspect_highlight_tokens: "EngLang: Inspect Highlight Tokens"',
        'inspect_highlight_token_at_cursor: "EngLang: Inspect Highlight Token at Cursor"',
        'copy_highlight_token_at_cursor: "EngLang: Copy Highlight Token at Cursor"',
        'copy_cursor: "EngLang: Copy Highlight Token at Cursor"'
    )) {
        if (-not $CommandHandlersSource.Contains($RequiredSemanticDebugToken)) {
            throw "VS Code semantic highlight inspection output missing contract token $RequiredSemanticDebugToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./semanticTokensProvider")') -or -not $SemanticTokensProviderSource.Contains("EngSemanticTokensProvider") -or -not $SemanticTokensProviderSource.Contains("snapshotDocumentSource")) {
        throw "VS Code extension must load semantic token provider orchestration from semanticTokensProvider.js"
    }
    if (-not $SemanticTokensProviderSource.Contains("onDidChangeSemanticTokens") -or -not $SemanticTokensProviderSource.Contains("refresh()") -or -not $SemanticTokensProviderSource.Contains("_onDidChangeSemanticTokens.fire()")) {
        throw "VS Code semantic token provider must notify VS Code when highlight settings change"
    }
    if (-not $ExtensionSource.Contains("semanticTokensProvider.refresh()") -or -not $ExtensionSource.Contains('affectsConfiguration("englang.semanticHighlighting.enabled")') -or -not $ExtensionSource.Contains("refreshVisibleSemanticSymbolDecorations")) {
        throw "VS Code extension must refresh semantic tokens and symbol decorations when semantic highlighting settings change"
    }
    if (-not $DecorationsSource.Contains('config.get("semanticHighlighting.enabled", true)') -or -not $DecorationsSource.Contains("refreshVisibleSemanticSymbolDecorations")) {
        throw "VS Code semantic symbol decorations must follow the semantic highlighting setting"
    }
    if (-not $SemanticProviderSource.Contains('require("./lspSemanticTokens")') -or -not $LspSemanticTokensSource.Contains("semanticTokensFromSnapshot") -or -not $LspSemanticTokensSource.Contains("semanticTokenRange") -or -not $LspSemanticTokensSource.Contains("semanticTokenDebugSample") -or -not $LspSemanticTokensSource.Contains("semanticTokenSelectors") -or -not $LspSemanticTokensSource.Contains("semanticTokenFallbackScopes") -or -not $LspSemanticTokensSource.Contains("semanticTokenUnmappedSelectors")) {
        throw "VS Code extension must share LSP semantic token conversion through lspSemanticTokens.js"
    }
    if ($ExtensionSource.Contains("class EngSemanticTokensProvider") -or $ExtensionSource.Contains("function semanticTokensFromSnapshot") -or $ExtensionSource.Contains("function semanticModifierBits") -or $ExtensionSource.Contains("function semanticTokenDebugSample")) {
        throw "VS Code extension must keep LSP semantic token conversion in lspSemanticTokens.js"
    }
    if ($CommandSourceCombined.Contains("No semantic token snapshot is available")) {
        throw "VS Code highlight inspection warning must use highlight wording"
    }
    foreach ($RequiredDiagnosticsToken in @(
        'require("./diagnosticsProvider")',
        "EngDiagnosticsController",
        "maybeCheck(document)",
        "scheduleChangedCheck",
        "clearPendingCheck",
        "checkActiveFile",
        "checkDocumentSource",
        "finishDocumentCheck",
        "toDiagnostics",
        "diagnosticCode",
        "diagnosticCodeTarget",
        "reference/cli/spec.md#diagnostic-codes",
        "top_level_execution_policy.md#args",
        "report_review.md#promoted-table-selection-and-transform-metadata",
        "diagnosticTags",
        "vscode.DiagnosticTag.Deprecated",
        "W-TABLE-LEGACY-SELECT-FIRST-ROW",
        "severityName",
        "firstLineRange",
        "lintOnSave",
        "lintOnChange",
        "diagnosticsRuntime",
        "diagnosticsRuntimeLabel",
        "live buffer check",
        "Problems source:",
        "diagnosticsSettingHint",
        "diagnosticSource",
        "function diagnosticSource",
        "eng/file",
        "eng/live",
        'Problems source: ${diagnosticSource(runtimeLabel)}',
        'diagnostics (${diagnosticSource(runtimeLabel)})',
        "source: diagnosticSource(runtimeLabel)",
        "diagnostic.source = source",
        "diagnostic.source = diagnosticSource(runtimeLabel)",
        "EngLang: Show Tooling Status",
        "diagnostics did not return editor JSON",
        "Tool failure:",
        "diagnosticsFailureDetail",
        "compactDiagnosticText",
        "stdout:",
        "See the EngLang output channel for stderr/stdout details"
    )) {
        if (-not $DiagnosticsSource.Contains($RequiredDiagnosticsToken)) {
            throw "VS Code extension missing diagnostics provider token $RequiredDiagnosticsToken"
        }
    }
    if ($ExtensionSource.Contains("function maybeCheck") -or $ExtensionSource.Contains("function scheduleChangedCheck") -or $ExtensionSource.Contains("function checkDocumentSource") -or $ExtensionSource.Contains("function finishDocumentCheck") -or $ExtensionSource.Contains("function toDiagnostics") -or $ExtensionSource.Contains("function toVscodeSeverity") -or $ExtensionSource.Contains("function firstLineRange")) {
        throw "VS Code extension must keep diagnostics helpers in diagnosticsProvider.js"
    }
    $DecorationSourceCombined = $ExtensionSource + "`n" + $DecorationsSource
    if (-not $ExtensionSource.Contains('require("./decorations")') -or -not $DecorationsSource.Contains("function createDecorationController")) {
        throw "VS Code extension must load decoration helpers from decorations.js"
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
        if (-not $DecorationSourceCombined.Contains($RequiredRiskDecorationToken)) {
            throw "VS Code extension missing review risk decoration token $RequiredRiskDecorationToken"
        }
    }
    $SemanticSymbolDecorationSource = $ExtensionSource + "`n" + $DecorationsSource + "`n" + $LspSemanticTokensSource
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
    foreach ($ForbiddenDecorationEntrypointToken in @(
        "function createReviewRiskDecorationTypes",
        "function createSemanticSymbolDecorationTypes",
        "function refreshVisibleReviewRiskDecorations",
        "function semanticSymbolDecorationOptions",
        "function semanticSymbolHoverMessage",
        "function reviewRiskDecorationOptions",
        "function setReviewRiskDecorationLine",
        "function fullLineRange"
    )) {
        if ($ExtensionSource.Contains($ForbiddenDecorationEntrypointToken)) {
            throw "VS Code extension must keep decoration helpers in decorations.js"
        }
    }
    $NavigationSource = $ExtensionSource + "`n" + $NavigationProvidersSource + "`n" + $LspNavigationSource + "`n" + $LspRequestsSource
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
    if (-not $ExtensionSource.Contains('require("./navigationProviders")') -or -not $NavigationProvidersSource.Contains("EngDocumentSymbolProvider") -or -not $NavigationProvidersSource.Contains("EngWorkspaceSymbolProvider") -or -not $NavigationProvidersSource.Contains("EngDefinitionProvider")) {
        throw "VS Code extension must load navigation provider orchestration from navigationProviders.js"
    }
    if (-not $NavigationProvidersSource.Contains('require("./lspNavigation")') -or -not $NavigationProvidersSource.Contains("workspaceSymbolInformationFromLsp") -or -not $NavigationProvidersSource.Contains("documentSymbolsFromSnapshot") -or -not $NavigationProvidersSource.Contains("definitionLocationFromLsp")) {
        throw "VS Code navigation providers must reuse shared LSP navigation conversion"
    }
    $FormattingSource = $ExtensionSource + "`n" + $FormattingProviderSource + "`n" + $LspRequestsSource
    foreach ($RequiredFormattingToken in @(
        "registerDocumentFormattingEditProvider",
        "registerDocumentRangeFormattingEditProvider",
        "EngFormattingProvider",
        "formatDocumentSource",
        "--format-stdin",
        "fullDocumentRange",
        "provideDocumentRangeFormattingEdits",
        "rangeFormattingEdit",
        "vscode.TextEdit.replace"
    )) {
        if (-not $FormattingSource.Contains($RequiredFormattingToken)) {
            throw "VS Code extension missing formatting token $RequiredFormattingToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./formattingProvider")') -or -not $FormattingProviderSource.Contains("EngFormattingProvider") -or -not $FormattingProviderSource.Contains("formatDocumentSource")) {
        throw "VS Code extension must load formatting provider orchestration from formattingProvider.js"
    }
    if ($ExtensionSource.Contains("class EngFormattingProvider") -or $ExtensionSource.Contains("function fullDocumentRange")) {
        throw "VS Code extension must keep formatting provider helpers in formattingProvider.js"
    }
    $FoldingSource = $ExtensionSource + "`n" + $FoldingRangeProviderSource
    foreach ($RequiredFoldingToken in @(
        "registerFoldingRangeProvider",
        "EngFoldingRangeProvider",
        "snapshotDocumentSource",
        "foldingRangesFromSnapshot",
        "foldingRangeFromSnapshot",
        "foldingRangeKindFromLsp",
        "new vscode.FoldingRange"
    )) {
        if (-not $FoldingSource.Contains($RequiredFoldingToken)) {
            throw "VS Code extension missing folding provider token $RequiredFoldingToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./foldingRangeProvider")') -or -not $FoldingRangeProviderSource.Contains("EngFoldingRangeProvider") -or -not $FoldingRangeProviderSource.Contains('require("./lspKinds")')) {
        throw "VS Code extension must load folding provider orchestration from foldingRangeProvider.js"
    }
    if ($ExtensionSource.Contains("class EngFoldingRangeProvider") -or $ExtensionSource.Contains("function foldingRangesFromSnapshot") -or $ExtensionSource.Contains("function foldingRangeFromSnapshot")) {
        throw "VS Code extension must keep folding provider helpers in foldingRangeProvider.js"
    }
    $QuickFixSource = $ExtensionSource + "`n" + $CodeActionProviderSource + "`n" + $LocalCodeActionsSource + "`n" + $LspCodeActionsSource + "`n" + $LspRequestsSource
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
        "mergeCodeActions",
        "codeActionEditKey",
        "shouldProvideQuickFixes",
        "context?.only",
        "codeActionKindIntersects",
        "isCancellationRequested",
        "return localActions()",
        "replacementIndexForDiagnostic(line.text, diagnostic, search)",
        "text.indexOf(search, boundedStart)",
        "edit.entries",
        "E-SYNTAX-DECL-001",
        "E-STRUCT-ARGS-001",
        "E-EQ-BOOL-001",
        "E-SCRIPT-001",
        "E-CMD-AMBIG-001",
        "E-CMD-UNKNOWN-VERB",
        "W-QTY-AMBIG-001",
        "W-STATS-SUM-001",
        "E-DIM-ADD-",
        "E-PUBLIC-ANNOTATION-001",
        "E-FS-CONFIRM-001",
        "E-FS-DELETE-001",
        "E-NET-INVALID-URL",
        "E-NET-BODY-METHOD",
        "E-NET-BODY-POLICY",
        "E-NET-HASH-MISMATCH",
        "W-NET-FIXTURE-ALIAS",
        "W-NET-RESPONSE-HASH-ALIAS",
        "W-NET-RESPONSE-STATUS-ALIAS",
        "E-IO-JSON-FIELD-ACCESS-001",
        "E-WITH-OPTION-001",
        "E-WITH-UNIT-001",
        "E-PRINT-FMT-001",
        "E-WRITE-FMT-001",
        "E-PRINT-FMT-002",
        "E-WRITE-FMT-002",
        "E-PRINT-FMT-004",
        "E-WRITE-FMT-004",
        "E-PRINT-FMT-003",
        "E-WRITE-FMT-003",
        "E-LOG-LEVEL-001",
        "E-REPORT-BINDING-001",
        "E-VALIDATE-BINDING-001",
        "E-SIDE-EFFECT-BINDING-001",
        "E-BLOCK-BINDING-001",
        "E-STATEMENT-BINDING-001",
        "E-OPTION-BINDING-001",
        "E-PROCESS-BINDING-001",
        "E-PROCESS-BINDING-002",
        "E-PROCESS-CMD-001",
        "E-ASSERT-001",
        "E-GOLDEN-001",
        "E-GOLDEN-002",
        "E-WHERE-FWD-001",
        "E-NAME-LOCAL-001",
        "E-UNC-ARGS-",
        "E-UNC-SOURCE-001",
        "E-UNC-SOURCE-002",
        "E-NET-RETRY-POLICY",
        "E-NET-TIMEOUT",
        "E-NET-BODY-SIZE-LIMIT",
        "E-PROCESS-RETRY-POLICY",
        "E-PROCESS-TIMEOUT",
        "E-PROCESS-ALLOW-FAILURE",
        "E-PROCESS-CWD-001",
        "E-PROCESS-ENV-001",
        "E-SAMPLING-COUNT-INVALID",
        "E-SAMPLING-SEED-INVALID",
        "E-SAMPLING-RANGE-UNIT",
        "E-WITH-UNCERTAINTY-POLICY-001",
        "E-WITH-UNCERTAINTY-SAMPLES-001",
        "E-WITH-UNCERTAINTY-SEED-001",
        "W-WITH-UNCERTAINTY-SEED-001",
        "E-ML-ARGS-001",
        "E-ML-ARGS-002",
        "E-ML-ARGS-003",
        "E-CACHE-KEY-NONDETERMINISTIC",
        "E-CACHE-DIR",
        "E-CACHE-TTL",
        "E-GOLDEN-001",
        "E-GOLDEN-002",
        "removeScriptWrapperAction",
        "quantityAnnotationActions",
        "heatRateSumAction",
        "Replace sum with integrate",
        "missingUnitActions",
        "schemaAnnotationAction",
        "fileMutationConfirmAction",
        "recursiveDeleteAction",
        "absoluteHttpUrlAction",
        "Replace URL with https://example.org",
        "httpBodyMethodAction",
        "Change HTTP method to post",
        "Replace request body with string literal",
        "expectedSha256Action",
        "expectedSha256FromDiagnostic",
        "optionKeyReplacementAction",
        "Rename fixture to offline_response",
        "diagnosticRangeReplacementAction",
        "Rename hash to response_hash",
        "W-NET-RESPONSE-STATUS-ALIAS",
        "response_source",
        "Rename status to response_source",
        "jsonReadPromotionAction",
        'Promote ${access.binding} before field access',
        "schemaNameFromBinding",
        "commandTargetParenthesesAction",
        "commandTargetFromDiagnostic",
        "commandStyleFunctionCallAction",
        "commandStyleFunctionCallEdit",
        "commandStyleVerbFromDiagnostic",
        "commandStyleFunctionCallArguments",
        "splitTopLevelCommandClauses",
        "topLevelCommandClausePositions",
        "Convert command-style call to function call",
        "withOptionAliasAction",
        "withOptionAliasFix",
        "unknownWithOptionName",
        "removeIncompatibleDisplayUnitAction",
        "Remove incompatible display unit option",
        "closeUnterminatedInterpolationAction",
        "unterminatedInterpolationClosePosition",
        "interpolationOpenIndex",
        "unescapedQuoteIndexAfter",
        "Close interpolation with }",
        "removeInterpolationDisplayUnitAction",
        "interpolationUnitRemovalRange",
        "formatSpecPrefixCanStandWithoutUnit",
        "lastBacktickPayload",
        "Remove incompatible interpolation unit",
        "logLevelInfoAction",
        "Set log level to info",
        "STATEMENT_ONLY_BINDING_CODES",
        "statementOnlyUnbindAction",
        "statementBindingPrefixRange",
        "Remove invalid binding prefix",
        "bindProcessResultAction",
        "Bind process result",
        "uniqueProcessBindingAction",
        "Rename process result to",
        "processCommandAction",
        "Add process command string",
        "wrapAssertionAction",
        "Wrap assertion in test block",
        "wrapGoldenAction",
        "Wrap golden check in test block",
        "goldenExpectedFileAction",
        "goldenBareExpectedStringRange",
        "Wrap golden expected path with file(...)",
        "reorderWhereLocalDefinitionAction",
        'Move ${name} definition before first use',
        "promoteWhereLocalAction",
        'Promote ${name} to top-level binding',
        "whereBlockDefiningBefore",
        "whereBlockMeaningfulLineCount",
        "fullLineBlockRange",
        "uncertaintyArgumentActions",
        "uncertaintySourceActions",
        "uncertaintySourceNameFromDiagnostic",
        "bindingExpressionRangeForName",
        "uncertaintyCallExampleFromDiagnostic",
        "uncertaintyCallRangeOnLine",
        "namedArgumentValueRange",
        "Define uncertainty source",
        "Add uncertainty source Q_source_unc",
        "to measured uncertainty source",
        "Set distribution kind to normal",
        "Set uncertainty method to linear",
        "Set uncertainty samples to 31",
        "Set uncertainty policy",
        "Set uncertainty samples",
        "Set uncertainty seed",
        "Add uncertainty seed: seed = 7",
        "Set deterministic cache key",
        "Set cache directory",
        "Set cache TTL to 1 h",
        "mlSourceActions",
        "Define ML",
        "Create ML split from",
        "Set model test split",
        "Set model seed",
        "Set model hidden layers",
        "Set model epochs",
        "modelOptionValueAction",
        "modelOptionFixes",
        "uncertaintySeedMissingAction",
        "withBlockContainingLine",
        "E-STDLIB-MODULE-UNKNOWN",
        "W-STDLIB-MODULE-PLANNED",
        "W-STDLIB-MODULE-INTERNAL",
        "stdlibModuleReplacementAction",
        "removeStdlibModuleImportAction",
        'Remove ${status} stdlib module import',
        "stdlibModuleNameFromDiagnostic",
        "stdlibModuleNamesFromCompletionItems",
        "closestStdlibModuleName",
        "editDistance",
        "Use plot y-axis option: unit y =",
        "Use plot x-axis option: unit x =",
        "Use confidence band option: confidence_band =",
        "Set process cwd",
        "Set process env",
        "E-SOLVE-SOLVER-UNSUPPORTED",
        "Set solve solver",
        "E-WRITE-002",
        "unsupportedWriteFormatActions",
        "Change write format to",
        "E-WRITE-STANDARD-TEXT-001",
        "Change writer to text",
        "E-WRITE-STANDARD-TEXT-OUTPUT",
        "standardTextOutputAction",
        "Add standard_text output path",
        "Set sample count",
        "Set sample seed",
        "optionQuickFix",
        "optionValueReplacementAction",
        "optionAssignmentRange",
        "removeEmptyInterpolationAction",
        "emptyInterpolationRange",
        "Remove empty interpolation",
        "convertUnresolvedInterpolationAction",
        "unresolvedInterpolationLiteralEdit",
        "Convert unresolved interpolation to literal text",
        "samplingRangeUnitAction",
        'Add unit ${fix.unit} to sample ${fix.endpoint} endpoint'
    )) {
        if (-not $QuickFixSource.Contains($RequiredQuickFixToken)) {
            throw "VS Code extension missing quick fix token $RequiredQuickFixToken"
        }
    }
    if (-not $ExtensionSource.Contains('require("./codeActionProvider")') -or -not $CodeActionProviderSource.Contains("EngCodeActionProvider") -or -not $CodeActionProviderSource.Contains("codeActionsForDocumentSource")) {
        throw "VS Code extension must load code action orchestration from codeActionProvider.js"
    }
    if (-not $CodeActionProviderSource.Contains('require("./localCodeActions")') -or -not $LocalCodeActionsSource.Contains("localCodeActions") -or -not $LocalCodeActionsSource.Contains("diagnosticCode")) {
        throw "VS Code code action provider must load local quick fix helpers from localCodeActions.js"
    }
    if (-not $ExtensionSource.Contains('completionItems: COMPLETION_ITEMS') -or -not $CodeActionProviderSource.Contains('this.completionItems = Array.isArray(options.completionItems)') -or -not $CodeActionProviderSource.Contains('completionItems: this.completionItems')) {
        throw "VS Code code action provider must pass generated completion catalog to local quick fixes"
    }
    if (-not $ExtensionSource.Contains('const UNIT_LABELS = catalogItemLabels(editorMetadata.syntaxCatalog.units)') -or -not $ExtensionSource.Contains('unitLabels: UNIT_LABELS') -or -not $CodeActionProviderSource.Contains('this.unitLabels = Array.isArray(options.unitLabels)') -or -not $CodeActionProviderSource.Contains('unitLabels: this.unitLabels') -or -not $LocalCodeActionsSource.Contains('missingUnitActions(document, diagnostic, options.unitLabels)') -or -not $LocalCodeActionsSource.Contains('unitLabelSet(unitLabels)')) {
        throw "VS Code missing-unit quick fixes must use generated unit catalog labels"
    }
    if (-not $ExtensionSource.Contains('const WORKFLOW_OPTION_LABELS = catalogItemLabels(editorMetadata.syntaxCatalog.workflow_options)') -or -not $ExtensionSource.Contains('workflowOptionLabels: WORKFLOW_OPTION_LABELS') -or -not $CodeActionProviderSource.Contains('this.workflowOptionLabels = Array.isArray(options.workflowOptionLabels)') -or -not $CodeActionProviderSource.Contains('workflowOptionLabels: this.workflowOptionLabels') -or -not $LocalCodeActionsSource.Contains('withOptionAliasAction(document, diagnostic, options.workflowOptionLabels)') -or -not $LocalCodeActionsSource.Contains('optionValueReplacementAction(') -or -not $LocalCodeActionsSource.Contains('modelOptionValueAction(document, diagnostic, code, options.workflowOptionLabels)') -or -not $LocalCodeActionsSource.Contains('knownWorkflowOptionNames(fix.optionNames, workflowOptionLabels)') -or -not $LocalCodeActionsSource.Contains('workflowOptionLabelSet(workflowOptionLabels)')) {
        throw "VS Code with-option quick fixes must use generated workflow option catalog labels"
    }
    foreach ($HiddenModelOptionAlias in @('optionName: "test_fraction"', 'optionName: "layers"')) {
        if ($LocalCodeActionsSource.Contains($HiddenModelOptionAlias)) {
            throw "VS Code model option quick fixes must not expose compatibility-only option aliases"
        }
    }
    if (-not $LocalCodeActionsSource.Contains('const code = stripLineComment(lineText)') -or -not $LocalCodeActionsSource.Contains('new RegExp(`^(\\s*)(${options})(\\s*=\\s*)(.*?)(\\s*)$`)')) {
        throw "VS Code option value quick fixes must strip # and // comments before replacing option values"
    }
    if (-not $CodeActionProviderSource.Contains('require("./lspCodeActions")') -or -not $LspCodeActionsSource.Contains("lspCodeActionsFromPayload") -or -not $LspCodeActionsSource.Contains("workspaceEditFromLspCodeAction")) {
        throw "VS Code code action provider must load LSP quick fix bridge helpers from lspCodeActions.js"
    }
    if (-not $LspKindsSource.Contains("symbolKindFromLsp") -or -not $LspKindsSource.Contains("completionKindFromLsp") -or -not $LspKindsSource.Contains("foldingRangeKindFromLsp")) {
        throw "VS Code extension must share LSP kind conversion through lspKinds.js"
    }
    if (-not $CompletionProviderSource.Contains('require("./lspKinds")')) {
        throw "VS Code completion provider must reuse shared LSP kind conversion"
    }
    foreach ($RequiredCompletionInsertToken in @("completionInsertTextFromCompletion", "completion.insert_snippet", "completion.insert", "new vscode.SnippetString")) {
        if (-not $CompletionProviderSource.Contains($RequiredCompletionInsertToken)) {
            throw "VS Code completion provider missing insert-text token $RequiredCompletionInsertToken"
        }
    }
    if (-not $FoldingRangeProviderSource.Contains('require("./lspKinds")')) {
        throw "VS Code folding provider must reuse shared LSP kind conversion"
    }
    if (-not $NavigationProvidersSource.Contains('require("./lspNavigation")') -or -not $LspNavigationSource.Contains("definitionLocationFromLsp") -or -not $LspNavigationSource.Contains("definitionLocationFromSnapshotSymbols") -or -not $LspNavigationSource.Contains("documentSymbolsFromSnapshot") -or -not $LspNavigationSource.Contains("workspaceSymbolInformationFromLsp") -or -not $LspNavigationSource.Contains("definitionNameCandidates")) {
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
    if ($ExtensionSource.Contains("class EngCodeActionProvider") -or $ExtensionSource.Contains("function mergeCodeActions") -or $ExtensionSource.Contains("function codeActionEditKey")) {
        throw "VS Code extension must keep quick fix orchestration in codeActionProvider.js"
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
    if ($ExtensionSource.Contains("class EngDocumentSymbolProvider") -or $ExtensionSource.Contains("class EngWorkspaceSymbolProvider") -or $ExtensionSource.Contains("class EngDefinitionProvider")) {
        throw "VS Code extension must keep navigation provider orchestration in navigationProviders.js"
    }
    if ($ExtensionSource.Contains("function definitionLocationFromLsp") -or $ExtensionSource.Contains("function definitionLocationFromSnapshotSymbols") -or $ExtensionSource.Contains("function workspaceSymbolInformationFromLsp") -or $ExtensionSource.Contains("function documentSymbolsFromSnapshot") -or $ExtensionSource.Contains("function definitionNameCandidates") -or $ExtensionSource.Contains("function identifierPathRangeAt")) {
        throw "VS Code extension must keep LSP navigation conversion in lspNavigation.js"
    }
    if ($ExtensionSource.Contains("function vscodeRangeFromLsp") -or $LspCodeActionsSource.Contains("function vscodeRangeFromLsp")) {
        throw "VS Code extension must keep LSP range conversion in lspRanges.js"
    }
    foreach ($RequiredDiagnosticsModeToken in @("function diagnosticsMode(document)", "function diagnosticsRuntime(document)", 'explicitlyConfiguredEngValue(config, "diagnosticsMode")', 'explicitlyConfiguredEngValue(config, "problemsSource")', 'return mode === "live" ? "lsp-snapshot" : "eng-cli"', "diagnosticsRuntimeLabel(runtimeMode)", "function refreshActiveDiagnosticsForSettings", "async function refreshAfterDiagnosticsModeCommand", "diagnosticController.checkActiveFile()", "diagnosticController.checkDocument(document)", "diagnosticController.clearDocumentDiagnostics", "file mode uses saved-file checks", "live typing diagnostics are disabled", "Diagnostics settings refresh", 'event.affectsConfiguration("englang.diagnosticsMode")', 'event.affectsConfiguration("englang.lintOnSave")', 'event.affectsConfiguration("englang.lintOnChange")')) {
        if (-not $ExtensionSource.Contains($RequiredDiagnosticsModeToken)) {
            throw "VS Code extension missing diagnostics mode compatibility token $RequiredDiagnosticsModeToken"
        }
    }
    if (-not $DiagnosticsProviderSource.Contains("clearDocumentDiagnostics(document") -or -not $DiagnosticsProviderSource.Contains("Problems cleared for") -or -not $DiagnosticsProviderSource.Contains("this.clearCachedReview(document)")) {
        throw "VS Code diagnostics provider must clear stale live Problems and cached review data when switching dirty editors back to file mode"
    }
    if (-not $ExtensionSource.Contains("clearCachedReview: (document) => reviewCache.delete(document.uri.fsPath)")) {
        throw "VS Code extension must clear cached live review data when switching dirty editors back to file mode"
    }
    if (-not $CommandHandlersSource.Contains("return picked.mode")) {
        throw "VS Code diagnostics mode command must return the selected mode so the active editor can refresh immediately"
    }
    $DiagnosticsModeEnum = @($Properties."englang.diagnosticsMode".enum)
    foreach ($RequiredDiagnosticsMode in @("file", "live")) {
        if ($DiagnosticsModeEnum -notcontains $RequiredDiagnosticsMode) {
            throw "VS Code extension diagnosticsMode missing enum value $RequiredDiagnosticsMode"
        }
    }
    if (@($Properties."englang.diagnosticsMode".enumDescriptions).Count -lt 2) {
        throw "VS Code extension diagnosticsMode must include user-facing enum descriptions"
    }
    foreach ($RequiredLegacyDiagnosticsModeToken in @('config.get("diagnosticsBackend", "eng-cli")', 'legacyBackend === "lsp-snapshot" ? "live" : "file"')) {
        if (-not $ExtensionSource.Contains($RequiredLegacyDiagnosticsModeToken)) {
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
    foreach ($RequiredLspQuickFixToken in @(
        "OptionValueQuickFix",
        "model_option_quick_fix",
        "model_option_quick_fix_option_names",
        "option_quick_fix_for_option",
        "lsp_ml_source_code_actions",
        "Define ML",
        "Create ML split from",
        "Set model test split",
        "Set model seed",
        "Set model hidden layers",
        "Set model epochs",
        "lsp_unsupported_write_format_code_actions",
        "Change write format to",
        "lsp_write_standard_text_output_code_action",
        "E-WRITE-STANDARD-TEXT-001",
        "Change writer to text",
        "Add standard_text output path",
        "lsp_sampling_range_unit_code_action",
        "Add unit {} to sample {} endpoint"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspQuickFixToken)) {
            throw "eng-lsp code action source missing quick fix token $RequiredLspQuickFixToken"
        }
    }
    foreach ($HiddenModelOptionAlias in @('"test_fraction"', '"layers"')) {
        if ($LspCliSource.Contains($HiddenModelOptionAlias)) {
            throw "eng-lsp model option quick fixes must not expose compatibility-only option aliases"
        }
    }
    if (-not $LspCliSource.Contains('line_comment_start(&line[value_start..])') -or -not $LspCliSource.Contains('option_assignment_range_preserves_trailing_line_comments')) {
        throw "eng-lsp option value quick fixes must preserve # and // comments when replacing option values"
    }
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
        "lsp_heat_rate_sum_code_action",
        "Replace sum with integrate",
        "lsp_missing_unit_code_actions",
        "lsp_schema_annotation_code_action",
        "lsp_file_mutation_confirm_code_action",
        "lsp_recursive_delete_code_action",
        "lsp_option_value_replacement_code_action",
        "lsp_with_option_alias_code_action",
        "with_option_alias_fix",
        "Use plot x-axis option: unit x =",
        "lsp_reorder_where_local_definition_code_action",
        "lsp_promote_where_local_code_action",
        "Promote {name} to top-level binding",
        "lsp_parenthesize_command_target_code_action",
        "lsp_stdlib_module_replacement_code_action",
        "lsp_remove_stdlib_module_import_code_action",
        "Remove {status} stdlib module import",
        "matching_block_end_line",
        "where_block_defining_before",
        "where_block_meaningful_line_count",
        "full_line_block_range",
        "E-SYNTAX-DECL-001",
        "E-STRUCT-ARGS-001",
        "E-SCRIPT-001",
        "E-EQ-BOOL-001",
        "E-CMD-AMBIG-001",
        "E-CMD-UNKNOWN-VERB",
        "lsp_command_style_function_call_code_action",
        "command_style_function_call_edit",
        "command_style_function_call_arguments",
        "split_top_level_command_clauses",
        "top_level_command_clause_positions",
        "Convert command-style call to function call",
        "E-WHERE-FWD-001",
        "E-NAME-LOCAL-001",
        "E-STDLIB-MODULE-UNKNOWN",
        "W-STDLIB-MODULE-PLANNED",
        "W-STDLIB-MODULE-INTERNAL",
        "W-QTY-AMBIG-001",
        "W-STATS-SUM-001",
        "E-DIM-ADD-",
        "E-PUBLIC-ANNOTATION-001",
        "E-FS-CONFIRM-001",
        "E-FS-DELETE-001",
        "E-NET-INVALID-URL",
        "lsp_absolute_http_url_code_action",
        "Replace URL with https://example.org",
        "E-NET-BODY-METHOD",
        "E-NET-BODY-POLICY",
        "lsp_http_body_method_code_action",
        "W-NET-RESPONSE-STATUS-ALIAS",
        "Rename status to response_source",
        "E-PRINT-FMT-001",
        "lsp_close_unterminated_interpolation_code_action",
        "Close interpolation with }",
        "E-PRINT-FMT-002",
        "lsp_remove_empty_interpolation_code_action",
        "Remove empty interpolation",
        "E-PRINT-FMT-003",
        "lsp_remove_interpolation_display_unit_code_action",
        "Remove incompatible interpolation unit",
        "E-PRINT-FMT-004",
        "lsp_convert_unresolved_interpolation_code_action",
        "Convert unresolved interpolation to literal text",
        "E-REPORT-BINDING-001",
        "E-VALIDATE-BINDING-001",
        "E-SIDE-EFFECT-BINDING-001",
        "E-BLOCK-BINDING-001",
        "E-STATEMENT-BINDING-001",
        "E-OPTION-BINDING-001",
        "lsp_statement_only_unbind_code_action",
        "statement_binding_prefix_range",
        "Remove invalid binding prefix",
        "Change HTTP method to post",
        "Replace request body with string literal",
        "E-NET-RETRY-POLICY",
        "E-WITH-UNCERTAINTY-POLICY-001",
        "E-WITH-UNCERTAINTY-SAMPLES-001",
        "E-WITH-UNCERTAINTY-SEED-001",
        "W-WITH-UNCERTAINTY-SEED-001",
        "E-GOLDEN-001",
        "E-GOLDEN-002",
        "lsp_wrap_golden_code_action",
        "Wrap golden check in test block",
        "lsp_uncertainty_seed_missing_code_action",
        "lsp_golden_expected_file_code_action",
        "golden_bare_expected_string_range",
        "with_block_containing_line",
        "Add uncertainty seed: seed = 7",
        "Wrap golden expected path with file(...)",
        "Set uncertainty policy",
        "Set uncertainty seed",
        "E-PROCESS-ALLOW-FAILURE",
        "E-SOLVE-SOLVER-UNSUPPORTED",
        "Set solve solver"
    )) {
        if (-not $LspCliSource.Contains($RequiredLspCodeActionToken)) {
            throw "eng-lsp CLI missing code action protocol token $RequiredLspCodeActionToken"
        }
    }
    foreach ($RequiredLspFormattingToken in @(
        "--format-stdin",
        "documentFormattingProvider",
        "documentRangeFormattingProvider",
        "textDocument/formatting",
        "textDocument/rangeFormatting",
        "formatting_edits_for_request",
        "range_formatting_edits_for_request",
        "full_document_range",
        "selected_line_range",
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
    if ($EditorMetadata.format -ne "eng-lsp-editor-metadata-v2") {
        throw "generated VS Code editor metadata returned unexpected format $($EditorMetadata.format)"
    }
    if ($GeneratedCompletions.format -ne "eng-lsp-editor-metadata-v2") {
        throw "generated VS Code completions returned unexpected format $($GeneratedCompletions.format)"
    }
    $MetadataCompletionLabels = @($EditorMetadata.completion_items | ForEach-Object { $_.label })
    $GeneratedCompletionLabels = @($GeneratedCompletions.completion_items | ForEach-Object { $_.label })
    Assert-SameStringSequence -Left $MetadataCompletionLabels -Right $GeneratedCompletionLabels -Description "VS Code generated completion item labels"
    if ($null -ne $EditorMetadata.PSObject.Properties["completion_seed"] -or $null -ne $EditorMetadata.PSObject.Properties["completion_seed_count"] -or $null -ne $GeneratedCompletions.PSObject.Properties["completion_seed"] -or $null -ne $GeneratedCompletions.PSObject.Properties["completion_seed_count"]) {
        throw "generated VS Code editor metadata must expose completion_items only; completion_seed was removed from the public editor metadata contract"
    }
    if ($MetadataCompletionLabels.Count -lt 100) {
        throw "generated VS Code completion item catalog is unexpectedly small: $($MetadataCompletionLabels.Count)"
    }
    if ($MetadataCompletionLabels -contains "fixture") {
        throw "generated VS Code completion items must not suggest legacy fixture option; use offline_response"
    }
    $StaleSamplingDetails = @($EditorMetadata.completion_items | Where-Object {
        $Detail = [string]$_.detail
        $Detail.Contains("seeded random sample table") -or $Detail.Contains("seeded uniform/random sample table")
    })
    if ($StaleSamplingDetails.Count -gt 0) {
        $StaleSamplingLabels = ($StaleSamplingDetails | ForEach-Object { $_.label }) -join ", "
        throw "generated VS Code completion details must describe deterministic sampling behavior, not seeded-only tables: $StaleSamplingLabels"
    }
    foreach ($StaticSnippetProperty in $Snippets.PSObject.Properties) {
        $StaticSnippetPrefix = [string]$StaticSnippetProperty.Value.prefix
        if ($MetadataCompletionLabels -contains $StaticSnippetPrefix) {
            throw "VS Code static snippet $($StaticSnippetProperty.Name) duplicates generated completion label $StaticSnippetPrefix"
        }
    }
    foreach ($RequiredGeneratedSnippet in @(
        @{ Label = "top workflow"; Tokens = @("args {", "report {"); RequiresSnippetKind = $true },
        @{ Label = "args block"; Tokens = @("args {", "CsvFile"); RequiresSnippetKind = $true },
        @{ Label = "log info"; Tokens = @("log info"); RequiresSnippetKind = $true },
        @{ Label = "http get"; Tokens = @("http get", "offline_response", "expected_sha256", "cache_key"); RequiresSnippetKind = $false },
        @{ Label = "http post"; Tokens = @("http post", "body =", "expected_sha256", "cache_key"); RequiresSnippetKind = $false },
        @{ Label = "sample lhs"; Tokens = @("sample lhs", "seed =", "uniform("); RequiresSnippetKind = $false },
        @{ Label = "sample grid"; Tokens = @("sample grid", "count =", "uniform("); RequiresSnippetKind = $false },
        @{ Label = "sample random"; Tokens = @("sample random", "seed =", "uniform("); RequiresSnippetKind = $false },
        @{ Label = "sample uniform"; Tokens = @("sample uniform", "seed =", "uniform("); RequiresSnippetKind = $false },
        @{ Label = "materialize cases"; Tokens = @("materialize cases"); RequiresSnippetKind = $false },
        @{ Label = "apply cases"; Tokens = @("apply", "over", "template = file", "overwrite = true"); RequiresSnippetKind = $false },
        @{ Label = "write sqlite"; Tokens = @('open sqlite ${2:args.database_target}', ".table", "transaction = commit"); RequiresSnippetKind = $false },
        @{ Label = "write standard_text"; Tokens = @("write standard_text", "output = join", "overwrite = true"); RequiresSnippetKind = $false },
        @{ Label = "run command"; Tokens = @("run command"); RequiresSnippetKind = $false },
        @{ Label = "test block"; Tokens = @("test", "within", "golden"); RequiresSnippetKind = $true },
        @{ Label = "schema csv"; Tokens = @('schema ${1:Sensor}', "HeatRate"); RequiresSnippetKind = $true },
        @{ Label = "plot line"; Tokens = @("plot", "unit y =", "title ="); RequiresSnippetKind = $true }
    )) {
        $Completion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq $RequiredGeneratedSnippet.Label }) | Select-Object -First 1
        if ($null -eq $Completion) {
            throw "generated VS Code editor metadata missing snippet completion $($RequiredGeneratedSnippet.Label)"
        }
        if ($RequiredGeneratedSnippet.RequiresSnippetKind -and $Completion.lsp_kind -ne 15) {
            throw "generated VS Code editor metadata snippet completion $($RequiredGeneratedSnippet.Label) must use LSP snippet kind"
        }
        $GeneratedSnippetBody = [string]$Completion.insert_snippet
        if (-not $GeneratedSnippetBody) {
            throw "generated VS Code editor metadata completion $($RequiredGeneratedSnippet.Label) missing insert_snippet"
        }
        foreach ($RequiredGeneratedSnippetToken in $RequiredGeneratedSnippet.Tokens) {
            if (-not $GeneratedSnippetBody.Contains($RequiredGeneratedSnippetToken)) {
                throw "generated VS Code editor metadata snippet $($RequiredGeneratedSnippet.Label) missing token $RequiredGeneratedSnippetToken"
            }
        }
        if ($RequiredGeneratedSnippet.Label -eq "write sqlite" -and $GeneratedSnippetBody.Contains('open sqlite file(${2:args.database_target})')) {
            throw "generated VS Code SQLite snippet must pass FilePath args directly instead of wrapping args.database_target in file(...)"
        }
    }
    foreach ($RequiredCompletion in @("records", "promote json records", "sample uniform", "sample latin-hypercube", "read json", "eng.table", "split")) {
        $Completion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq $RequiredCompletion }) | Select-Object -First 1
        if ($null -eq $Completion) {
            throw "generated VS Code editor metadata missing completion item $RequiredCompletion"
        }
        if ($null -eq $Completion.lsp_kind) {
            throw "generated VS Code editor metadata completion item $RequiredCompletion missing lsp_kind"
        }
    }
    $SampleUniformCompletion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq "sample uniform" }) | Select-Object -First 1
    if (-not [string]$SampleUniformCompletion.insert_snippet -or -not ([string]$SampleUniformCompletion.insert_snippet).Contains("sample uniform`nwith {")) {
        throw "generated VS Code editor metadata sample uniform completion must include a with-block insert_snippet"
    }
    $ReadJsonCompletion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq "read json" }) | Select-Object -First 1
    if ($ReadJsonCompletion.insert -ne "read json args.config" -or $ReadJsonCompletion.insert_snippet -ne 'read json ${1:args.config}') {
        throw "generated VS Code editor metadata read json completion must include insert and insert_snippet"
    }
    $LinearOperatorCompletion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq "LinearOperator[From -> To]" }) | Select-Object -First 1
    if ($LinearOperatorCompletion.insert_snippet -ne 'LinearOperator[${1:From} -> ${2:To}]') {
        throw "generated VS Code editor metadata LinearOperator completion must include insert_snippet"
    }
    foreach ($RequiredHyphenatedWorkflowBuiltin in @("latin-hypercube")) {
        $HyphenatedWorkflowBuiltin = @($EditorMetadata.syntax_catalog.hyphenated_workflow_builtins | Where-Object { $_ -eq $RequiredHyphenatedWorkflowBuiltin }) | Select-Object -First 1
        if ($null -eq $HyphenatedWorkflowBuiltin) {
            throw "generated VS Code editor metadata missing hyphenated workflow builtin $RequiredHyphenatedWorkflowBuiltin"
        }
    }
    foreach ($RequiredSyntaxConstant in @("monte_carlo", "source_linear_terms", "lhs", "latin_hypercube")) {
        $SyntaxConstant = @($EditorMetadata.syntax_catalog.constants | Where-Object { $_ -eq $RequiredSyntaxConstant }) | Select-Object -First 1
        if ($null -eq $SyntaxConstant) {
            throw "generated VS Code editor metadata missing syntax constant $RequiredSyntaxConstant"
        }
    }
    foreach ($RequiredOperatorWord in @("between", "within", "matches")) {
        $OperatorWord = @($EditorMetadata.syntax_catalog.operator_words | Where-Object { $_ -eq $RequiredOperatorWord }) | Select-Object -First 1
        if ($null -eq $OperatorWord) {
            throw "generated VS Code editor metadata missing operator word $RequiredOperatorWord"
        }
    }
    $RequiredKeywordGroups = @("import", "deprecated", "declaration", "function", "test", "block", "modifier", "report", "validation", "side_effect", "external_boundary", "solver", "workflow")
    foreach ($RequiredKeywordGroup in $RequiredKeywordGroups) {
        $KeywordGroupProperty = $EditorMetadata.syntax_catalog.keyword_groups.PSObject.Properties[$RequiredKeywordGroup]
        if ($null -eq $KeywordGroupProperty -or @($KeywordGroupProperty.Value).Count -eq 0) {
            throw "generated VS Code editor metadata missing keyword group $RequiredKeywordGroup"
        }
    }
    foreach ($RequiredSyntaxKeyword in @("histogram", "parity", "residuals", "return", "if", "else")) {
        $SyntaxKeyword = @($EditorMetadata.syntax_catalog.keywords | Where-Object { $_ -eq $RequiredSyntaxKeyword }) | Select-Object -First 1
        if ($null -eq $SyntaxKeyword) {
            throw "generated VS Code editor metadata missing syntax keyword $RequiredSyntaxKeyword"
        }
    }
    $CompilerLexerKeywords = [regex]::Matches($CompilerLexerSource, '"([a-z_]+)"\s*=>\s*Some\(Keyword::') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
    if (@($CompilerLexerKeywords).Count -eq 0) {
        throw "compiler lexer keyword registry guard found no keywords"
    }
    foreach ($CompilerLexerKeyword in $CompilerLexerKeywords) {
        if (@($EditorMetadata.syntax_catalog.keywords) -notcontains $CompilerLexerKeyword) {
            throw "generated VS Code editor metadata missing compiler lexer keyword $CompilerLexerKeyword"
        }
    }
    foreach ($HiddenModelOptionAlias in @("x", "y", "test_fraction", "layers")) {
        $ModelOptionAlias = @($EditorMetadata.syntax_catalog.workflow_options | Where-Object { $_.label -eq $HiddenModelOptionAlias }) | Select-Object -First 1
        if ($null -ne $ModelOptionAlias) {
            throw "generated VS Code editor metadata must not suggest compatibility-only model option alias $HiddenModelOptionAlias"
        }
        $CompletionAlias = @($EditorMetadata.completion_items | Where-Object { $_.label -eq $HiddenModelOptionAlias }) | Select-Object -First 1
        if ($null -ne $CompletionAlias) {
            throw "generated VS Code editor metadata completion_items must not suggest compatibility-only model option alias $HiddenModelOptionAlias"
        }
        $LegacyWorkflowOptionAlias = @($EditorMetadata.syntax_catalog.legacy_workflow_option_aliases | Where-Object { $_ -eq $HiddenModelOptionAlias }) | Select-Object -First 1
        if ($null -eq $LegacyWorkflowOptionAlias) {
            throw "generated VS Code editor metadata must keep compatibility-only model option alias $HiddenModelOptionAlias as a highlight-only legacy workflow option alias"
        }
    }
    foreach ($NativeModuleCompletion in @("eng.net", "eng.cache", "eng.report", "eng.plot", "eng.uncertainty")) {
        $ModuleCompletion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq $NativeModuleCompletion }) | Select-Object -First 1
        if ($null -eq $ModuleCompletion) {
            throw "generated VS Code editor metadata missing module completion $NativeModuleCompletion"
        }
        $ModuleCompletionDetail = [string]$ModuleCompletion.detail
        if (-not $ModuleCompletionDetail.StartsWith("Native: ")) {
            throw "generated VS Code editor metadata module completion $NativeModuleCompletion must use short Native: completion detail"
        }
        if ($ModuleCompletionDetail.Contains("Native workflow support:") -or $ModuleCompletionDetail.Contains("broader") -or $ModuleCompletionDetail.Contains("remains planned")) {
            throw "generated VS Code editor metadata module completion $NativeModuleCompletion must not expose full status labels or planned-scope tails"
        }
    }
    foreach ($RequiredHttpResponseField in @("body", "response_source", "status_code", "query_string", "url_with_query")) {
        $HttpResponseField = @($EditorMetadata.syntax_catalog.http_response_fields | Where-Object { $_.label -eq $RequiredHttpResponseField }) | Select-Object -First 1
        if ($null -eq $HttpResponseField) {
            throw "generated VS Code editor metadata missing HTTP response field $RequiredHttpResponseField"
        }
        if ([string]::IsNullOrWhiteSpace($HttpResponseField.detail)) {
            throw "generated VS Code editor metadata HTTP response field $RequiredHttpResponseField missing detail"
        }
    }
    if (@($EditorMetadata.syntax_catalog.http_response_fields | Where-Object { $_.label -eq "status" }).Count -gt 0) {
        throw "generated VS Code editor metadata must not suggest response.status; use response_source"
    }
    foreach ($RequiredCoverageResultField in @("actual_count", "expected_count", "missing_count", "max_gap_hours")) {
        $CoverageResultField = @($EditorMetadata.syntax_catalog.coverage_result_fields | Where-Object { $_.label -eq $RequiredCoverageResultField }) | Select-Object -First 1
        if ($null -eq $CoverageResultField) {
            throw "generated VS Code editor metadata missing coverage result field $RequiredCoverageResultField"
        }
        if ([string]::IsNullOrWhiteSpace($CoverageResultField.detail)) {
            throw "generated VS Code editor metadata coverage result field $RequiredCoverageResultField missing detail"
        }
    }
    foreach ($RequiredTableField in @("rows", "row_count", "column_count", "schema_name")) {
        $TableField = @($EditorMetadata.syntax_catalog.table_fields | Where-Object { $_.label -eq $RequiredTableField }) | Select-Object -First 1
        if ($null -eq $TableField) {
            throw "generated VS Code editor metadata missing generic table field $RequiredTableField"
        }
        if ([string]::IsNullOrWhiteSpace($TableField.detail)) {
            throw "generated VS Code editor metadata generic table field $RequiredTableField missing detail"
        }
    }
    foreach ($RequiredSampleTableField in @("sample_count", "method", "seed", "parameter_count")) {
        $SampleTableField = @($EditorMetadata.syntax_catalog.sample_table_fields | Where-Object { $_.label -eq $RequiredSampleTableField }) | Select-Object -First 1
        if ($null -eq $SampleTableField) {
            throw "generated VS Code editor metadata missing sample table field $RequiredSampleTableField"
        }
        if ([string]::IsNullOrWhiteSpace($SampleTableField.detail)) {
            throw "generated VS Code editor metadata sample table field $RequiredSampleTableField missing detail"
        }
    }
    foreach ($RequiredDbConnectionField in @("tables_written", "table_count", "row_count", "status")) {
        $DbConnectionField = @($EditorMetadata.syntax_catalog.db_connection_fields | Where-Object { $_.label -eq $RequiredDbConnectionField }) | Select-Object -First 1
        if ($null -eq $DbConnectionField) {
            throw "generated VS Code editor metadata missing DB connection field $RequiredDbConnectionField"
        }
        if ([string]::IsNullOrWhiteSpace($DbConnectionField.detail)) {
            throw "generated VS Code editor metadata DB connection field $RequiredDbConnectionField missing detail"
        }
    }
    foreach ($RequiredCaseTableField in @("case_count", "pending_count", "status")) {
        $CaseTableField = @($EditorMetadata.syntax_catalog.case_table_fields | Where-Object { $_.label -eq $RequiredCaseTableField }) | Select-Object -First 1
        if ($null -eq $CaseTableField) {
            throw "generated VS Code editor metadata missing case table field $RequiredCaseTableField"
        }
        if ([string]::IsNullOrWhiteSpace($CaseTableField.detail)) {
            throw "generated VS Code editor metadata case table field $RequiredCaseTableField missing detail"
        }
    }
    foreach ($RequiredCaseOutputTableField in @("expected_count", "rendered_count", "blocked_count", "manifest_count")) {
        $CaseOutputTableField = @($EditorMetadata.syntax_catalog.case_output_table_fields | Where-Object { $_.label -eq $RequiredCaseOutputTableField }) | Select-Object -First 1
        if ($null -eq $CaseOutputTableField) {
            throw "generated VS Code editor metadata missing case output table field $RequiredCaseOutputTableField"
        }
        if ([string]::IsNullOrWhiteSpace($CaseOutputTableField.detail)) {
            throw "generated VS Code editor metadata case output table field $RequiredCaseOutputTableField missing detail"
        }
    }
    $PlannedCaseOutputTableField = @($EditorMetadata.syntax_catalog.case_output_table_fields | Where-Object { $_.label -eq "planned_count" }) | Select-Object -First 1
    if ($null -ne $PlannedCaseOutputTableField) {
        throw "generated VS Code editor metadata must not suggest compatibility-only case_inputs.planned_count; use expected_count"
    }
    foreach ($RequiredCaseResultCollectionTableField in @("collected_count", "missing_count", "blocked_count", "status")) {
        $CaseResultCollectionTableField = @($EditorMetadata.syntax_catalog.case_result_collection_table_fields | Where-Object { $_.label -eq $RequiredCaseResultCollectionTableField }) | Select-Object -First 1
        if ($null -eq $CaseResultCollectionTableField) {
            throw "generated VS Code editor metadata missing case result collection table field $RequiredCaseResultCollectionTableField"
        }
        if ([string]::IsNullOrWhiteSpace($CaseResultCollectionTableField.detail)) {
            throw "generated VS Code editor metadata case result collection table field $RequiredCaseResultCollectionTableField missing detail"
        }
    }
    foreach ($RequiredModelField in @("status", "train_count", "rmse", "model_artifact_hash")) {
        $ModelField = @($EditorMetadata.syntax_catalog.model_fields | Where-Object { $_.label -eq $RequiredModelField }) | Select-Object -First 1
        if ($null -eq $ModelField) {
            throw "generated VS Code editor metadata missing model field $RequiredModelField"
        }
        if ([string]::IsNullOrWhiteSpace($ModelField.detail)) {
            throw "generated VS Code editor metadata model field $RequiredModelField missing detail"
        }
    }
    foreach ($RequiredPredictionTableField in @("case_count", "status", "output_column", "confidence_column")) {
        $PredictionTableField = @($EditorMetadata.syntax_catalog.prediction_table_fields | Where-Object { $_.label -eq $RequiredPredictionTableField }) | Select-Object -First 1
        if ($null -eq $PredictionTableField) {
            throw "generated VS Code editor metadata missing prediction table field $RequiredPredictionTableField"
        }
        if ([string]::IsNullOrWhiteSpace($PredictionTableField.detail)) {
            throw "generated VS Code editor metadata prediction table field $RequiredPredictionTableField missing detail"
        }
    }
    $GeneratedSemanticTypes = @($EditorMetadata.semantic_token_legend.token_types)
    $GeneratedSemanticModifiers = @($EditorMetadata.semantic_token_legend.token_modifiers)
    if (-not $LspSource.Contains("fn hover_display_unit") -or -not $LspSource.Contains('Display unit: `{display_unit}`')) {
        throw "eng-lsp hover markdown must suppress empty display-unit labels"
    }
    $LspSemanticTypes = Read-RustStringSliceConst -Source $LspSource -Name "SEMANTIC_TOKEN_TYPES"
    $LspSemanticModifiers = Read-RustStringSliceConst -Source $LspSource -Name "SEMANTIC_TOKEN_MODIFIERS"
    Assert-SameStringSequence -Left $GeneratedSemanticTypes -Right $LspSemanticTypes -Description "VS Code generated/LSP semantic token types"
    Assert-SameStringSequence -Left $GeneratedSemanticModifiers -Right $LspSemanticModifiers -Description "VS Code generated/LSP semantic token modifiers"
    $StandardSemanticModifiers = @("declaration", "definition", "readonly", "static", "local", "imported", "defaultLibrary", "deprecated", "documentation")
    foreach ($Modifier in $LspSemanticModifiers) {
        if ($StandardSemanticModifiers -notcontains $Modifier -and $SemanticModifiers -notcontains $Modifier) {
            throw "VS Code package.json missing custom semantic token modifier from LSP legend: $Modifier"
        }
    }

    $VscodeJavaScriptPaths = @(
        $ExtensionJsPath,
        $ArtifactOpenersPath,
        $CommandHandlersPath,
        $DecorationsPath,
        $CompletionProviderPath,
        $CodeActionProviderPath,
        $DiagnosticsProviderPath,
        $FoldingRangeProviderPath,
        $FormattingProviderPath,
        $HoverProviderPath,
        $NavigationProvidersPath,
        $SemanticTokensProviderPath,
        $LocalCodeActionsPath,
        $LspCodeActionsPath,
        $LspKindsPath,
        $LspNavigationPath,
        $LspRangesPath,
        $LspRequestsPath,
        $LspSemanticTokensPath,
        $ArtifactRegistryPath,
        $EditorMetadataLoaderPath,
        $ExecutionProfilesPath,
        $ModuleStatusPath,
        $RuntimeDiscoveryPath,
        $ReviewPanelRendererPath
    )
    Invoke-JavaScriptSyntaxCheck -Paths $VscodeJavaScriptPaths -Label "VS Code extension"

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
    $CompilerSemanticSourcePath = Join-Path $RepoRoot "crates\eng_compiler\src\semantic.rs"
    $LspSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\lib.rs"
    if (-not (Test-Path $TauriConfigPath)) {
        throw "missing portable native IDE config at $TauriConfigPath"
    }
    if (-not (Test-Path $TauriMainPath)) {
        throw "missing portable native IDE backend at $TauriMainPath"
    }
    if (-not (Test-Path $TauriUiIndexPath)) {
        throw "missing portable native IDE static frontend at $TauriUiIndexPath"
    }
    if (-not (Test-Path $TauriUiAppPath)) {
        throw "missing portable native IDE frontend script at $TauriUiAppPath"
    }
    if (-not (Test-Path $TauriUiStylesPath)) {
        throw "missing portable native IDE frontend styles at $TauriUiStylesPath"
    }
    if (-not (Test-Path $CompilerSemanticSourcePath)) {
        throw "missing compiler semantic source at $CompilerSemanticSourcePath"
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
        'placeholder="Filter diagnostics"',
        'placeholder="check, run, cd <dir>, or EngLang statement"',
        "Supports check, run, reset, clear, cd <dir>, and one-line EngLang statements.",
        "filteredProblems",
        "moduleCategory",
        "moduleQueryInput",
        "filteredModules",
        "data-module-category",
        "data-problem-line",
        "data-problem-column",
        "data-problem-start-character",
        "data-problem-end-character",
        "function selectProblemRange(row)",
        "selectProblemRange(row)",
        "function selectSourceCharacterRange(line, startCharacter, endCharacter)",
        "diag.startCharacter ?? diag.start_character",
        "diag.endCharacter ?? diag.end_character",
        "function problemRangeCell(diag)",
        "diag.rangeText",
        "diag.range_text",
        '`column ${diag.column}`',
        '`range: ${diag?.rangeText || diag?.range_text || "-"}`',
        "data-copy-problem-index",
        "activeProblemCode",
        "problemCopyButton",
        "copyProblemDiagnostic",
        "copyVisibleProblemsBtn",
        "copyVisibleProblems",
        "problemCopyText",
        "RUN_HISTORY_STORAGE_PREFIX",
        "data-open-file-path",
        "data-open-path",
        "ide_open_path",
        "EDITOR_INDENT",
        'const EDITOR_INDENT = "    "',
        "EDITOR_PAIR_CLOSE",
        "formatBtn",
        "formatCurrent",
        "Already formatter-clean",
        "editorHighlight",
        "renderHighlightedSource",
        "renderLexicalHighlightedLine",
        "renderLexicalString",
        "renderLexicalInterpolation",
        "scanInterpolationEnd",
        "lexicalClassForWord",
        "lexicalCompletionClass",
        'case "property":',
        'return "hl-property"',
        'case "method":',
        'return "hl-method"',
        'case "constant":',
        'return "hl-constant"',
        "syntaxCatalog",
        "normalizeSyntaxCatalog",
        "buildLexicalCatalog",
        "operatorWords",
        "operator_words",
        "normalized.constants",
        "keywords: stringArray(source.keywords)",
        "const keywordSet = new Set([",
        "constants: new Set(normalized.constants)",
        "normalized.operatorWords",
        "operatorWords: new Set(normalized.operatorWords)",
        "normalized.units",
        "legacyUnitAliases",
        "legacy_unit_aliases",
        "normalized.legacyUnitAliases",
        "const unitLabels = uniqueStrings([",
        "...normalized.legacyUnitAliases",
        "hyphenatedWorkflowBuiltins",
        "hyphenated_workflow_builtins",
        "legacyWorkflowBuiltinAliases",
        "legacy_workflow_builtin_aliases",
        "...normalized.legacyWorkflowBuiltinAliases",
        "legacyWorkflowOptionAliases",
        "legacy_workflow_option_aliases",
        "...normalized.legacyWorkflowOptionAliases",
        "lexicalUnitPattern",
        "hl-doc-comment",
        'rest.startsWith("///")',
        "toggleEditorLineComment",
        "isLineCommented",
        'rest.startsWith("//")',
        '\/\/(?!\/)',
        "indentEditorSelection",
        "outdentEditorSelection",
        "insertEditorNewlineWithIndent",
        "handleEditorPairKey",
        "insertEditorPair",
        "skipEditorClosingPair",
        "deleteEmptyEditorPair",
        "insertClosingBraceWithIndent",
        "selectedLineEditRange",
        "syncEditorManualEdit",
        "localMemberCompletionCandidates",
        "memberCompletionContextFromPrefix",
        "memberCompletionItemsForFields",
        "argsFieldCompletionsFromSource",
        "schemaBindingFieldCompletionsFromSource",
        "workflowBindingFieldCompletionsFromSource",
        "apply\s*\(",
        "workflowFieldsForBinding",
        "workflowMemberCompletionFields",
        "httpResponseFields",
        "coverageResultFields",
        "tableFields",
        "promote\s+(?:csv|toml|json(?:\s+records)?)",
        "check\s+coverage",
        "isCoverageResultLikeReceiver",
        "isTableLikeReceiver",
        "sampleTableFields",
        "latin[_-]hypercube|grid|random|uniform",
        "caseTableFields",
        "caseOutputTableFields",
        "normalized.includes(""rendered"")",
        "normalized.includes(""blocked"")",
        "schemaFieldsForBinding",
        "promotedSchemaBindingsFromSource",
        "firstBlockBodyFromSource",
        "renderHighlightPanel",
        "renderHighlightPanelStatus",
        "function semanticTokenOverlaps(tokens)",
        "function semanticTokenLineOverlaps(lineIndex)",
        "lineOverlaps: semanticTokenLineOverlaps(position.line)",
        "function renderCaretLineOverlapCell(overlaps)",
        "function renderCaretLineOverlapNotice(overlaps)",
        "Line Overlaps",
        'parts.push(`overlaps ${lineOverlaps.length}`)',
        "function renderSemanticOverlapSummary(overlaps)",
        'Overlaps ${overlaps.length}',
        "No overlapping semantic highlight ranges for the current check.",
        "Check current file to refresh precise highlight ranges.",
        "No role-aware highlights were returned for the current check.",
        "Filter hides all current highlights.",
        "Highlight data is current. Showing",
        "renderNearbySemanticTokenRows",
        "nearestTokens",
        "semanticTokensNearCaret",
        "semanticTokenCaretDistance",
        "caretDistance: semanticTokenCaretDistance",
        "columnByte >= start && columnByte < end",
        "Nearby Highlights",
        "Select Nearby",
        'near ${tokenLabel(nearestToken)}',
        "No exact highlight at the caret.",
        "highlightTokenQuery",
        "highlightTokenQueryInput",
        "clearHighlightTokenFilter",
        "filteredSemanticTokens",
        "semanticTokenSearchText",
        "semanticTokenText",
        "highlightFilterChip",
        "semanticTokenLegendChip",
        "<th>Text</th>",
        "semanticTokenPayload",
        "semanticTokens",
        "byteOffsetToCodeUnit",
        "cursorInsight",
        "renderCursorInsight",
        "editorBracketMatch",
        "editorBracketAtCaret",
        "matchingBracketOffset",
        "scanBracketForward",
        "scanBracketBackward",
        "unmatched",
        "bindCursorInsightActions",
        "renderCursorInsightActions",
        "data-show-highlight-panel",
        "semanticTokenAtCaret",
        "hoverForSemanticToken",
        "sourceTokenButton",
        "sourceTokenActions",
        "Copy Text",
        "Copy Range",
        "Copy Selector",
        "Copy token selector",
        "<th>Actions</th>",
        "sourceTokenCopyButton",
        "bindSourceTokenCopyButtons",
        "copySourceTokenRange",
        "semanticTokenPrimarySelector",
        "semanticTokenForRange",
        'mode === "selector"',
        "copyTextToClipboard",
        "data-copy-source-token",
        "setStatus",
        "selectSourceTokenRange",
        "data-source-token-line",
        "sourceLineRange",
        "sourceColumnStart",
        "const targetByte = Math.max(0, Math.trunc(columnNumber) - 1)",
        "byteOffsetToCodeUnit(String(lineText ||",
        "sourceLineValue",
        "sourceColumnValue",
        "data-source-column",
        "source_line",
        "sourceLine",
        "source_column",
        "sourceColumn",
        "variableSourceCell",
        "variable-source-line",
        "codeUnitToByteOffset",
        "Timestamp",
        "Output Root",
        "Write Records",
        "Training Plans",
        "Prediction Runs",
        "Case Runs",
        "processResultsPanelTitle",
        "Process Results (0 external processes)",
        "No external process executions recorded.",
        "External Process Results"
    )) {
        if (-not $IdeUiSource.Contains($RequiredIdeToken)) {
            throw "Native IDE UI missing contract token $RequiredIdeToken"
        }
    }
    if ($IdeUiSource.Contains('(?:#|\/\/) ?')) {
        throw "Native IDE comment toggling must not treat /// documentation comments as ordinary // comments"
    }
    foreach ($UniqueNativeCompletionFunction in @(
        "localMemberCompletionCandidates",
        "memberCompletionContextFromPrefix",
        "argsFieldCompletionsFromSource",
        "schemaBindingFieldCompletionsFromSource",
        "workflowBindingFieldCompletionsFromSource",
        "workflowFieldsForBinding",
        "workflowMemberCompletionFields",
        "schemaFieldCompletionsFromBody"
    )) {
        $FunctionDeclarationCount = [regex]::Matches($IdeUiSource, "function\s+$UniqueNativeCompletionFunction\s*\(").Count
        if ($FunctionDeclarationCount -ne 1) {
            throw "Native IDE completion UI must declare $UniqueNativeCompletionFunction exactly once"
        }
    }
    foreach ($RequiredNativeMemberCompletionToken in @(
        "dbConnectionFields: catalogFieldItems(source.dbConnectionFields ?? source.db_connection_fields)",
        "coverageResultFields: catalogFieldItems(source.coverageResultFields ?? source.coverage_result_fields)",
        "tableFields: catalogFieldItems(source.tableFields ?? source.table_fields)",
        "modelFields: catalogFieldItems(source.modelFields ?? source.model_fields)",
        "predictionTableFields: catalogFieldItems(source.predictionTableFields ?? source.prediction_table_fields)",
        "function receiverLookupCandidates(receiver)",
        "function firstMappedFieldsForReceiver(fieldMap, receiverCandidates)",
        "context.receiverCandidates",
        'normalized.split(".").filter(Boolean).pop()',
        "workflowCatalog.dbConnectionFields",
        "workflowCatalog.coverageResultFields",
        "workflowCatalog.tableFields",
        "workflowCatalog.modelFields",
        "workflowCatalog.predictionTableFields",
        "normalizedCatalog.dbConnectionFields",
        "normalizedCatalog.coverageResultFields",
        "normalizedCatalog.tableFields",
        "normalizedCatalog.modelFields",
        "normalizedCatalog.predictionTableFields",
        "function isDbConnectionLikeReceiver(receiver)",
        "function isCoverageResultLikeReceiver(receiver)",
        "function isTableLikeReceiver(receiver)",
        "function isModelLikeReceiver(receiver)",
        "function isPredictionTableLikeReceiver(receiver)"
    )) {
        if (-not $IdeUiSource.Contains($RequiredNativeMemberCompletionToken)) {
            throw "Native IDE member completions must consume generated public field catalogs and dotted receiver fallbacks: $RequiredNativeMemberCompletionToken"
        }
    }
    if (-not $IdeUiSource.Contains("latin[_-]hypercube|grid|random|uniform")) {
        throw "Native IDE sample table completions must recognize sample uniform fallback bindings"
    }
    if (-not $IdeUiSource.Contains("const SIDE_TABS = [") -or -not $IdeUiSource.Contains('SIDE_TABS.map((tab) => sideTabButton(tab.key, tab.label)).join(""')) {
        throw "Native IDE side tabs must be driven by the ordered SIDE_TABS registry"
    }
    $ExpectedNativeIdeSideTabOrder = @(
        '{ key: "variables", label: "Variables" }',
        '{ key: "units", label: "Units" }',
        '{ key: "schema", label: "Schema" }',
        '{ key: "time", label: "Time" }',
        '{ key: "tables", label: "Tables" }',
        '{ key: "reads", label: "Reads" }',
        '{ key: "plot", label: "Plot" }',
        '{ key: "review", label: "Review" }',
        '{ key: "quality", label: "Quality" }',
        '{ key: "checks", label: "Checks" }',
        '{ key: "effects", label: "Effects" }',
        '{ key: "network", label: "Network" }',
        '{ key: "artifacts", label: "Artifacts" }',
        '{ key: "workflow", label: "Workflow" }',
        '{ key: "case", label: "Case" }',
        '{ key: "model", label: "Model" }',
        '{ key: "db", label: "DB" }',
        '{ key: "run", label: "Run" }',
        '{ key: "modules", label: "Modules" }',
        '{ key: "objects", label: "Objects" }',
        '{ key: "assembly", label: "Assembly" }',
        '{ key: "kernels", label: "Kernel" }',
        '{ key: "highlight", label: "Highlight" }'
    )
    $PreviousNativeIdeSideTabIndex = -1
    foreach ($ExpectedNativeIdeSideTab in $ExpectedNativeIdeSideTabOrder) {
        $NativeIdeSideTabIndex = $IdeUiSource.IndexOf($ExpectedNativeIdeSideTab)
        if ($NativeIdeSideTabIndex -lt 0) {
            throw "Native IDE side tab order missing $ExpectedNativeIdeSideTab"
        }
        if ($NativeIdeSideTabIndex -lt $PreviousNativeIdeSideTabIndex) {
            throw "Native IDE side tab order should keep units, review, workflow, and artifact panels before advanced panels"
        }
        $PreviousNativeIdeSideTabIndex = $NativeIdeSideTabIndex
    }
    if (-not $IdeUiSource.Contains("function renderUnitsPanel()") -or -not $IdeUiSource.Contains("Review Units") -or -not $IdeUiSource.Contains("Unit Conversions")) {
        throw "Native IDE side tabs must expose a dedicated Units panel before Schema"
    }
    foreach ($ForbiddenNativeIdeSideTabLabel in @(
        'sideTabButton("variables", "Vars")',
        'sideTabButton("network", "Net")',
        'sideTabButton("workflow", "Flow")',
        'sideTabButton("objects", "Obj")',
        'sideTabButton("assembly", "Asm")'
    )) {
        if ($IdeUiSource.Contains($ForbiddenNativeIdeSideTabLabel)) {
            throw "Native IDE side tab label must use clear wording instead of $ForbiddenNativeIdeSideTabLabel"
        }
    }
    if ($IdeUiSource.Contains('<div class="panel-title compact">External Process Results</div>')) {
        throw "Native IDE Effects panel must compute process result wording from process_count"
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
        "No semantic highlight at the caret.",
        "No highlight at the caret.",
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
    if (-not $IdeUiSource.Contains("Response Source") -or -not $IdeUiSource.Contains("response_source") -or -not $IdeUiSource.Contains("responseSource")) {
        throw "Native IDE network panel must label response source separately from generic status"
    }
    foreach ($RequiredModuleWordingToken in @("moduleStatusDisplay", "moduleStatusDetail", "module.status_label", "module.status_detail", "moduleBackingLabel", "Compiler/runtime", "No executable backing")) {
        if (-not $IdeUiSource.Contains($RequiredModuleWordingToken)) {
            throw "Native IDE module wording missing token $RequiredModuleWordingToken"
        }
    }
    if ($IdeUiSource.Contains('${escapeHtml(module.status || "-")} / ${escapeHtml(module.backing || "-")}')) {
        throw "Native IDE Modules view must not display raw registry status/backing keys"
    }
    if ($IdeUiSource.Contains('return "Native workflow support";')) {
        throw "Native IDE module fallback label should stay short; use compiler status_label for full registry wording"
    }
    foreach ($RequiredBehaviorStatusToken in @(
        "delay_call_runtime_buffer_pending_integration",
        "predictor_call_contract_pending_integration",
        "external_behavior_wrapper_pending_integration",
        "predictor_contract_metadata",
        "external_behavior_contract_metadata",
        "safe_repro_profile_policy_metadata"
    )) {
        if (-not $IdeUiSource.Contains($RequiredBehaviorStatusToken)) {
            throw "Native IDE behavior status labels missing current artifact status $RequiredBehaviorStatusToken"
        }
    }
    foreach ($ForbiddenBehaviorStatusToken in @(
        "delay_call_runtime_buffer_seed_not_integrated",
        "predictor_call_contract_seed_not_integrated",
        "external_behavior_wrapper_seed_not_integrated",
        "predictor_contract_metadata_seed",
        "external_behavior_contract_metadata_seed",
        "safe_repro_profile_policy_seed"
    )) {
        if ($IdeUiSource.Contains($ForbiddenBehaviorStatusToken)) {
            throw "Native IDE behavior status labels must not expose legacy seed status $ForbiddenBehaviorStatusToken"
        }
    }
    $CompilerSemanticSource = Get-Content -LiteralPath $CompilerSemanticSourcePath -Raw
    foreach ($RequiredAssemblyBalanceStatusToken in @(
        "balanced_metadata",
        "underdetermined_metadata",
        "overdetermined_metadata"
    )) {
        if (-not $CompilerSemanticSource.Contains($RequiredAssemblyBalanceStatusToken)) {
            throw "Compiler assembly balance status missing current token $RequiredAssemblyBalanceStatusToken"
        }
    }
    foreach ($ForbiddenAssemblyBalanceStatusToken in @(
        "balanced_metadata_seed",
        "underdetermined_seed",
        "overdetermined_seed"
    )) {
        if ($CompilerSemanticSource.Contains($ForbiddenAssemblyBalanceStatusToken)) {
            throw "Compiler assembly balance status must not expose legacy seed status $ForbiddenAssemblyBalanceStatusToken"
        }
    }
    if ($IdeUiSource.Contains("FALLBACK_LEXICAL_KEYWORDS")) {
        throw "Native IDE keyword fallback must use syntax_catalog keywords and keyword_groups instead of a hardcoded JS list"
    }
    if ($IdeUiSource.Contains("FALLBACK_LEXICAL_CONSTANTS")) {
        throw "Native IDE constant fallback must use syntax_catalog.constants instead of a hardcoded JS list"
    }
    if ($IdeUiSource.Contains("FALLBACK_LEXICAL_OPERATOR_WORDS")) {
        throw "Native IDE operator fallback must use syntax_catalog.operator_words instead of a hardcoded JS list"
    }
    if ($IdeUiSource.Contains("FALLBACK_LEXICAL_UNITS")) {
        throw "Native IDE unit fallback must use syntax_catalog.units and syntax_catalog.legacy_unit_aliases instead of a hardcoded JS list"
    }
    $IdeUiStyles = Get-Content -LiteralPath $TauriUiStylesPath -Raw
    foreach ($RequiredIdeStyle in @("run-history-table", "status-pill", "status-pill.completed", "status-pill.blocked", "problem-query", "problem-row", "problem-message", "problem-actions", "problem-copy-button", "module-toolbar", "module-query", "editor-highlight", "hl-keyword", "hl-interpolation", "hl-constant", "hl-punctuation", "hl-mod-unit", "hl-mod-solver", "hl-mod-validation", "hl-mod-report", "hl-mod-sideEffect", "hl-mod-external", "hl-mod-riskHigh", "semantic-token-table", ".semantic-token-table th:last-child", "token-chip", "token-filter-chip", "token-range-button", "cursor-insight", "variable-source-line")) {
        if (-not $IdeUiStyles.Contains($RequiredIdeStyle)) {
            throw "Native IDE UI missing contract style $RequiredIdeStyle"
        }
    }
    foreach ($ForbiddenGroupedIdeHighlightStyle in @(
        ".hl-mod-axis,`n.hl-mod-timeseries",
        ".hl-mod-validation,`n.hl-mod-report",
        ".hl-mod-sideEffect,`n.hl-mod-external"
    )) {
        if ($IdeUiStyles.Contains($ForbiddenGroupedIdeHighlightStyle)) {
            throw "Native IDE semantic highlight styles must keep role colors distinct: $ForbiddenGroupedIdeHighlightStyle"
        }
    }
    $LspSource = Get-Content -LiteralPath $LspSourcePath -Raw
    $LspLanguageConstants = Read-RustStringSliceConst -Source $LspSource -Name "LANGUAGE_CONSTANT_KEYWORDS"
    foreach ($RequiredNativeIdeCatalogConstant in @("asc", "desc", "metadata_ready", "warnings_present", "diagnostics_present", "implicit_euler_dae", "trapezoidal")) {
        if ($LspLanguageConstants -notcontains $RequiredNativeIdeCatalogConstant) {
            throw "LSP language constants missing native IDE lexical catalog constant $RequiredNativeIdeCatalogConstant"
        }
    }
    if (-not $LspSource.Contains("fn hover_display_unit") -or -not $LspSource.Contains('Display unit: `{display_unit}`')) {
        throw "eng-lsp hover markdown must suppress empty display-unit labels"
    }
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
    foreach ($RequiredIdeBackendToken in @("eng_lsp", "semantic_tokens", "hovers", "editor_payload_view", "snapshot_from_report_with_source", "hover_json", "format_source", "ide_format", "FormatView", "native_ide_format_uses_compiler_formatter", "editor_completion_items", "hyphenated_workflow_builtins", "latin-hypercube", "CompletionView::from_lsp", ".insert", "unwrap_or_else(|| completion.label.clone())", "native_ide_completions_use_lsp_editor_items", "check_view_surfaces_lsp_semantic_tokens", "one-line EngLang statement such as", "cd <dir>", "diagnostic_view_from_lsp", "diagnostic_view_from_parts", 'range_text: format!("L{line}:C{column}-C{end_column}")')) {
        if (-not $IdeMainSource.Contains($RequiredIdeBackendToken)) {
            throw "Native IDE backend missing contract token $RequiredIdeBackendToken"
        }
    }
    foreach ($ForbiddenNativeIdeCompletionToken in @("BASE_COMPLETION_KEYWORDS", "PUBLIC_TYPE_COMPLETIONS", "WORKFLOW_BUILTIN_COMPLETIONS", "WORKFLOW_OPTION_COMPLETIONS")) {
        if ($IdeMainSource.Contains($ForbiddenNativeIdeCompletionToken)) {
            throw "Native IDE backend must use eng_lsp editor completion items instead of $ForbiddenNativeIdeCompletionToken"
        }
    }
    foreach ($ForbiddenNativeIdeFixtureToken in @("python run.py", '"target": "python"')) {
        if ($IdeMainSource.Contains($ForbiddenNativeIdeFixtureToken)) {
            throw "Native IDE backend fixture must not expose legacy Python workflow marker $ForbiddenNativeIdeFixtureToken"
        }
    }
    Invoke-JavaScriptSyntaxCheck -Paths @($TauriUiAppPath) -Label "Native IDE app"
    Invoke-Native $cargo "check" "-p" "eng_ide"
    Invoke-Native $cargo "run" "-p" "eng_ide" "--" "--smoke"
    Assert-VscodeExtensionContract
    Write-Host "IDE check passed."
}

function Assert-VscodeSemanticFallbackCoverage {
    param(
        [Parameter(Mandatory = $true)]
        [string] $LspExecutable
    )

    $ExtensionRoot = Join-Path $RepoRoot "tools\vscode-englang"
    $PackageJsonPath = Join-Path $ExtensionRoot "package.json"
    if (-not (Test-Path $PackageJsonPath)) {
        throw "missing VS Code extension package.json at $PackageJsonPath"
    }
    if (-not (Test-Path $LspExecutable)) {
        throw "missing eng-lsp executable at $LspExecutable"
    }

    $Package = Get-Content -LiteralPath $PackageJsonPath -Raw | ConvertFrom-Json
    $SemanticScopeRule = @($Package.contributes.semanticTokenScopes | Where-Object { $_.language -eq "englang" }) | Select-Object -First 1
    if ($null -eq $SemanticScopeRule) {
        throw "VS Code extension missing englang semantic token scope mappings"
    }
    $ScopeSelectors = @{}
    foreach ($ScopeProperty in @($SemanticScopeRule.scopes.PSObject.Properties)) {
        $FallbackScopes = @($ScopeProperty.Value |
            ForEach-Object { [string]$_ } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        $ScopeSelectors[$ScopeProperty.Name] = $FallbackScopes
    }

    $SnapshotRoots = @(
        (Join-Path $RepoRoot "examples")
        (Join-Path $RepoRoot "tools\vscode-englang\test\grammar-fixtures")
    ) | Where-Object { Test-Path $_ }
    $SourceFiles = @()
    foreach ($SnapshotRoot in $SnapshotRoots) {
        $SourceFiles += @(Get-ChildItem -LiteralPath $SnapshotRoot -Recurse -Filter "*.eng" -File)
    }
    if (@($SourceFiles).Count -eq 0) {
        throw "VS Code semantic fallback coverage check found no EngLang source files"
    }

    $ObservedSelectors = @{}
    $MissingSelectors = @{}
    $EmptyFallbackSelectors = @{}
    $TokenCount = 0
    foreach ($SourceFile in $SourceFiles) {
        $SnapshotOutput = & $LspExecutable "--snapshot" $SourceFile.FullName
        if ($LASTEXITCODE -ne 0) {
            throw "eng-lsp semantic fallback coverage snapshot failed for $($SourceFile.FullName)"
        }
        $Snapshot = ($SnapshotOutput | Out-String).Trim() | ConvertFrom-Json
        foreach ($Token in @($Snapshot.semantic_tokens.tokens)) {
            $TokenCount += 1
            $TokenType = [string]$Token.type
            if ([string]::IsNullOrWhiteSpace($TokenType)) {
                continue
            }
            $Selectors = @()
            foreach ($Modifier in @($Token.modifiers)) {
                if (-not [string]::IsNullOrWhiteSpace([string]$Modifier)) {
                    $Selectors += "$TokenType.$Modifier"
                }
            }
            $Selectors += $TokenType
            foreach ($Selector in @($Selectors | Select-Object -Unique)) {
                if ($ObservedSelectors.ContainsKey($Selector)) {
                    $ObservedSelectors[$Selector] = [int]$ObservedSelectors[$Selector] + 1
                } else {
                    $ObservedSelectors[$Selector] = 1
                }
                if (-not $ScopeSelectors.ContainsKey($Selector)) {
                    if ($MissingSelectors.ContainsKey($Selector)) {
                        $MissingSelectors[$Selector] = [int]$MissingSelectors[$Selector] + 1
                    } else {
                        $MissingSelectors[$Selector] = 1
                    }
                } elseif (@($ScopeSelectors[$Selector]).Count -eq 0) {
                    if ($EmptyFallbackSelectors.ContainsKey($Selector)) {
                        $EmptyFallbackSelectors[$Selector] = [int]$EmptyFallbackSelectors[$Selector] + 1
                    } else {
                        $EmptyFallbackSelectors[$Selector] = 1
                    }
                }
            }
        }
    }
    if ($ObservedSelectors.Count -eq 0) {
        throw "VS Code semantic fallback coverage check found no semantic selectors"
    }
    if ($MissingSelectors.Count -gt 0) {
        $MissingSummary = ($MissingSelectors.GetEnumerator() |
            Sort-Object -Property Value -Descending |
            Select-Object -First 20 |
            ForEach-Object { "$($_.Name)=$($_.Value)" }) -join ", "
        throw "VS Code semantic token scope fallback map is missing observed selector(s): $MissingSummary"
    }
    if ($EmptyFallbackSelectors.Count -gt 0) {
        $EmptyFallbackSummary = ($EmptyFallbackSelectors.GetEnumerator() |
            Sort-Object -Property Value -Descending |
            Select-Object -First 20 |
            ForEach-Object { "$($_.Name)=$($_.Value)" }) -join ", "
        throw "VS Code semantic token scope fallback map has observed selector(s) with no fallback scopes: $EmptyFallbackSummary"
    }
    Write-Host "VS Code semantic fallback coverage passed. Checked $(@($SourceFiles).Count) snapshot(s), $($ObservedSelectors.Count) selector(s), $TokenCount semantic token(s)."
}

function Invoke-LspCheck {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    $LspCliSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\main.rs"
    $LspCliSource = Get-Content -LiteralPath $LspCliSourcePath -Raw
    foreach ($RequiredPersistentLspToken in @(
        "struct DocumentState",
        "type Documents = HashMap<String, DocumentState>",
        "document_state_from_notification",
        "document_version_from_request",
        '"textDocumentSync": {',
        '"openClose": true',
        '"save": { "includeText": true }',
        '"method": "textDocument/publishDiagnostics"',
        'params.insert("version".to_owned(), json!(version))'
    )) {
        if (-not $LspCliSource.Contains($RequiredPersistentLspToken)) {
            throw "eng-lsp CLI missing persistent document/version diagnostics token $RequiredPersistentLspToken"
        }
    }

    Invoke-Native $cargo "test" "-p" "eng_lsp" "--" "--nocapture"
    Invoke-Native $cargo "run" "-p" "eng_lsp" "--" "--smoke"
    Invoke-Native $cargo "run" "-p" "eng_lsp" "--" "--snapshot-check" "examples\official\01_csv_plot\main.eng"
    $EditorMetadataOutput = & $cargo "run" "-p" "eng_lsp" "--quiet" "--" "--editor-metadata"
    if ($LASTEXITCODE -ne 0) {
        throw "eng-lsp --editor-metadata failed with exit code $LASTEXITCODE"
    }
    $EditorMetadata = ($EditorMetadataOutput | Out-String).Trim() | ConvertFrom-Json
    if ($EditorMetadata.format -ne "eng-lsp-editor-metadata-v2") {
        throw "eng-lsp --editor-metadata returned unexpected format $($EditorMetadata.format)"
    }
    foreach ($RequiredCompletion in @("records", "promote json records", "sample uniform", "sample latin-hypercube", "read json", "eng.table", "split")) {
        $Completion = @($EditorMetadata.completion_items | Where-Object { $_.label -eq $RequiredCompletion }) | Select-Object -First 1
        if ($null -eq $Completion) {
            throw "eng-lsp --editor-metadata missing completion item $RequiredCompletion"
        }
    }
    foreach ($RequiredModifier in @("workflowStep", "unit", "quantity", "solver")) {
        if (@($EditorMetadata.semantic_token_legend.token_modifiers) -notcontains $RequiredModifier) {
            throw "eng-lsp --editor-metadata missing semantic token modifier $RequiredModifier"
        }
    }
    $LspExecutable = Join-Path $RepoRoot "target\debug\eng-lsp.exe"
    Assert-VscodeSemanticFallbackCoverage -LspExecutable $LspExecutable
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
    <Description xml:space="preserve">EngLang editor tooling with diagnostics, hover, completion, and program execution.</Description>
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

function Get-VscodeUserExtensionsDirectory {
    return Join-Path $env:USERPROFILE ".vscode\extensions"
}

function Get-LocalVscodeVsixPath {
    $PackageRoot = Join-Path $RepoRoot "dist\local-vscode"
    return Join-Path (Join-Path $PackageRoot "tools") (Get-VsixFileName)
}

function Get-InstalledVscodeEnglangExtensionPaths {
    $ExtensionRoot = Get-VscodeUserExtensionsDirectory
    if (-not (Test-Path -LiteralPath $ExtensionRoot -PathType Container)) {
        return @()
    }
    return @(Get-ChildItem -LiteralPath $ExtensionRoot -Directory -Filter "englang.englang-*" -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
}

function Get-RunningVscodeProcessSummaries {
    return @(Get-Process -Name "Code" -ErrorAction SilentlyContinue | ForEach-Object { "$($_.ProcessName)#$($_.Id)" })
}

function Format-ByteSize {
    param([Parameter(Mandatory = $true)][long] $Bytes)

    if ($Bytes -ge 1GB) {
        return "{0:N1} GB" -f ($Bytes / 1GB)
    }
    if ($Bytes -ge 1MB) {
        return "{0:N1} MB" -f ($Bytes / 1MB)
    }
    if ($Bytes -ge 1KB) {
        return "{0:N1} KB" -f ($Bytes / 1KB)
    }
    return "$Bytes B"
}

function Get-VscodeExtensionInstallUpdatedTime {
    param([Parameter(Mandatory = $true)][string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return $null
    }
    try {
        $Newest = (Get-Item -LiteralPath $Path).LastWriteTimeUtc
        foreach ($InstalledFile in @(Get-ChildItem -LiteralPath $Path -Recurse -File -ErrorAction SilentlyContinue)) {
            if ($InstalledFile.LastWriteTimeUtc -gt $Newest) {
                $Newest = $InstalledFile.LastWriteTimeUtc
            }
        }
        return $Newest
    } catch {
        return $null
    }
}

function Format-VscodeTimestamp {
    param([Parameter(Mandatory = $true)] $Timestamp)

    return $Timestamp.ToLocalTime().ToString("yyyy-MM-dd HH:mm:ss")
}

function Get-VscodeExtensionInstallSummary {
    param([Parameter(Mandatory = $true)][string] $Path)

    $UpdatedTime = Get-VscodeExtensionInstallUpdatedTime -Path $Path
    $UpdatedSuffix = ""
    if ($null -ne $UpdatedTime) {
        $UpdatedSuffix = ", updated $(Format-VscodeTimestamp -Timestamp $UpdatedTime)"
    }
    $PackageJsonPath = Join-Path $Path "package.json"
    if (Test-Path -LiteralPath $PackageJsonPath -PathType Leaf) {
        try {
            $Package = Get-Content -LiteralPath $PackageJsonPath -Raw | ConvertFrom-Json
            if ($null -ne $Package.version -and [string]$Package.version -ne "") {
                return "$Path (version $($Package.version)$UpdatedSuffix)"
            }
        } catch {
            return "$Path (package.json unreadable$UpdatedSuffix)"
        }
    }
    if ($UpdatedSuffix -ne "") {
        return "$Path ($($UpdatedSuffix.TrimStart(', ')))"
    }
    return $Path
}

function Get-VscodeExtensionFreshnessSummary {
    param(
        [Parameter(Mandatory = $true)][string] $VsixPath,
        [Parameter(Mandatory = $true)] $InstalledExtensions
    )

    if (-not (Test-Path -LiteralPath $VsixPath -PathType Leaf)) {
        return "Install freshness: unknown - build the VSIX with .\dev.bat vscode-package."
    }
    if ($InstalledExtensions.Count -eq 0) {
        return "Install freshness: not installed - run .\dev.bat vscode-install or install the VSIX manually."
    }

    $InstalledUpdates = @()
    foreach ($InstalledExtension in $InstalledExtensions) {
        $UpdatedTime = Get-VscodeExtensionInstallUpdatedTime -Path $InstalledExtension
        if ($null -ne $UpdatedTime) {
            $InstalledUpdates += [PSCustomObject]@{ Path = $InstalledExtension; Updated = $UpdatedTime }
        }
    }
    if ($InstalledUpdates.Count -eq 0) {
        return "Install freshness: unknown - installed EngLang extension timestamp could not be read."
    }

    $VsixItem = Get-Item -LiteralPath $VsixPath
    $NewestInstalled = $InstalledUpdates | Sort-Object Updated -Descending | Select-Object -First 1
    if ($VsixItem.LastWriteTimeUtc -gt $NewestInstalled.Updated.AddSeconds(1)) {
        $VsixUpdated = Format-VscodeTimestamp -Timestamp $VsixItem.LastWriteTimeUtc
        $InstalledUpdated = Format-VscodeTimestamp -Timestamp $NewestInstalled.Updated
        return "Install freshness: update available - built VSIX is newer than installed EngLang extension (VSIX $VsixUpdated, installed $InstalledUpdated); close all VS Code windows and run .\dev.bat vscode-install."
    }
    return "Install freshness: current - installed EngLang extension is at least as new as the built VSIX."
}

function Get-LatestVscodePackageInputTime {
    $InputPaths = @(
        (Join-Path $RepoRoot "tools\vscode-englang"),
        (Join-Path $RepoRoot "target\release\eng.exe"),
        (Join-Path $RepoRoot "target\release\eng-lsp.exe")
    )
    $Latest = $null

    foreach ($InputPath in $InputPaths) {
        if (Test-Path -LiteralPath $InputPath -PathType Leaf) {
            $Item = Get-Item -LiteralPath $InputPath
            if ($null -eq $Latest -or $Item.LastWriteTimeUtc -gt $Latest) {
                $Latest = $Item.LastWriteTimeUtc
            }
        } elseif (Test-Path -LiteralPath $InputPath -PathType Container) {
            foreach ($Item in @(Get-ChildItem -LiteralPath $InputPath -Recurse -File -ErrorAction SilentlyContinue)) {
                if ($null -eq $Latest -or $Item.LastWriteTimeUtc -gt $Latest) {
                    $Latest = $Item.LastWriteTimeUtc
                }
            }
        }
    }

    return $Latest
}

function Get-VscodePackageFreshnessSummary {
    param([Parameter(Mandatory = $true)][string] $VsixPath)

    if (-not (Test-Path -LiteralPath $VsixPath -PathType Leaf)) {
        return "Package freshness: missing - run .\dev.bat vscode-package."
    }
    $LatestInput = Get-LatestVscodePackageInputTime
    if ($null -eq $LatestInput) {
        return "Package freshness: unknown - VS Code package input timestamps could not be read."
    }

    $VsixItem = Get-Item -LiteralPath $VsixPath
    if ($LatestInput -gt $VsixItem.LastWriteTimeUtc.AddSeconds(1)) {
        $InputUpdated = Format-VscodeTimestamp -Timestamp $LatestInput
        $VsixUpdated = Format-VscodeTimestamp -Timestamp $VsixItem.LastWriteTimeUtc
        return "Package freshness: rebuild available - VS Code extension source or release binaries are newer than the built VSIX (inputs $InputUpdated, VSIX $VsixUpdated); run .\dev.bat vscode-package."
    }
    return "Package freshness: current - built VSIX is at least as new as VS Code extension source and release binaries."
}

function Get-LocalVscodeVsixSummary {
    param([Parameter(Mandatory = $true)][string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    $Item = Get-Item -LiteralPath $Path
    $Size = Format-ByteSize -Bytes $Item.Length
    $Updated = $Item.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")
    return "$Path (version $(Get-WorkspaceVersion), $Size, updated $Updated)"
}

function Invoke-VscodeStatus {
    $Code = Get-VscodeCli
    $VsixPath = Get-LocalVscodeVsixPath
    $InstalledExtensions = Get-InstalledVscodeEnglangExtensionPaths
    $RunningVscode = Get-RunningVscodeProcessSummaries

    if ($null -eq $Code) {
        Write-Host "VS Code CLI: not found"
    } else {
        Write-Host "VS Code CLI: $Code"
    }

    $VsixSummary = Get-LocalVscodeVsixSummary -Path $VsixPath
    if ($null -ne $VsixSummary) {
        Write-Host "Built VSIX: $VsixSummary"
    } else {
        Write-Host "Built VSIX: missing - run .\dev.bat vscode-package"
    }

    if ($InstalledExtensions.Count -eq 0) {
        Write-Host "Installed EngLang extension(s): none"
    } else {
        $InstalledSummaries = @($InstalledExtensions | ForEach-Object { Get-VscodeExtensionInstallSummary -Path $_ })
        Write-Host "Installed EngLang extension(s): $($InstalledSummaries -join ', ')"
    }

    if ($RunningVscode.Count -eq 0) {
        Write-Host "Running VS Code process(es): none"
    } else {
        Write-Host "Running VS Code process(es): $($RunningVscode -join ', ')"
    }

    Write-Host (Get-VscodePackageFreshnessSummary -VsixPath $VsixPath)
    Write-Host (Get-VscodeExtensionFreshnessSummary -VsixPath $VsixPath -InstalledExtensions $InstalledExtensions)

    if ($InstalledExtensions.Count -gt 0 -and $RunningVscode.Count -gt 0) {
        Write-Host "Install preflight: blocked - close all VS Code windows before reinstalling EngLang."
    } elseif ($null -eq $Code) {
        Write-Host "Install preflight: manual VSIX install required because the code CLI was not found."
    } else {
        Write-Host "Install preflight: ready - run .\dev.bat vscode-install."
    }
}

function Assert-VscodeInstallPreflight {
    $InstalledExtensions = Get-InstalledVscodeEnglangExtensionPaths
    if ($InstalledExtensions.Count -eq 0) {
        return
    }
    $RunningVscode = Get-RunningVscodeProcessSummaries
    if ($RunningVscode.Count -eq 0) {
        return
    }
    $InstalledList = $InstalledExtensions -join ", "
    $ProcessList = $RunningVscode -join ", "
    $ExistingVsixPath = Get-LocalVscodeVsixPath
    $VsixHint = ""
    if (Test-Path -LiteralPath $ExistingVsixPath -PathType Leaf) {
        $VsixHint = " Existing built VSIX: $ExistingVsixPath."
    }
    throw "Close all VS Code windows before reinstalling EngLang. Existing extension folder(s): $InstalledList. Running VS Code process(es): $ProcessList. To only build the VSIX, run .\dev.bat vscode-package.$VsixHint"
}

function Invoke-VscodePackage {
    Set-DevEnvironment
    $cargo = Get-Cargo
    if ($null -eq $cargo) {
        Write-Host "Cargo not found. Run .\dev.bat setup."
        exit 1
    }
    Invoke-Native $cargo "build" "--release" "-p" "eng_cli" "-p" "eng_lsp"
    Assert-VscodeSemanticFallbackCoverage -LspExecutable (Join-Path $RepoRoot "target\release\eng-lsp.exe")
    $PackageRoot = Join-Path $RepoRoot "dist\local-vscode"
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    Invoke-IdePackage -PackageRoot $PackageRoot
    $VsixPath = Get-LocalVscodeVsixPath
    Write-Host "Local VS Code VSIX ready: $VsixPath"
    return $VsixPath
}

function Invoke-VscodeInstall {
    Assert-VscodeInstallPreflight
    $VsixPath = Invoke-VscodePackage
    $Code = Get-VscodeCli
    if ($null -eq $Code) {
        throw "VS Code CLI not found. Install the VSIX manually from $VsixPath, or add the VS Code 'code' command to PATH."
    }
    $InstallWorkingDirectory = Join-Path $CacheHome "vscode-install"
    $InstallUserDataDirectory = Join-Path $InstallWorkingDirectory "user-data"
    $InstallExtensionsDirectory = Get-VscodeUserExtensionsDirectory
    New-Item -ItemType Directory -Force -Path $InstallUserDataDirectory, $InstallExtensionsDirectory | Out-Null
    try {
        Invoke-NativeInDirectory -WorkingDirectory $InstallWorkingDirectory -FilePath $Code "--user-data-dir" $InstallUserDataDirectory "--extensions-dir" $InstallExtensionsDirectory "--install-extension" $VsixPath "--force"
    } catch {
        throw "VS Code extension install failed. Close all VS Code windows and rerun .\dev.bat vscode-install, or install the generated VSIX manually from $VsixPath. VS Code CLI user data and logs, if any, are under $InstallUserDataDirectory. $($_.Exception.Message)"
    }
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
        @{ Kind = "h1"; Text = "3. Native IDE Workflow" },
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
        @{ Kind = "body"; Text = "This release supports the documented core workflows: CSV promote, unit-aware TimeSeries calculations, PlotSpec/SVG output, review/report artifacts, package smoke, official examples, standalone packaging, and the portable native IDE smoke path. Advanced solver, compatibility, diagnostic, and internal regression fixtures remain source-repository material, not portable package tutorials. Public solver support, general nonlinear solving, DAE solving, production multi-domain component graph solving, native JIT execution, broad uncertainty and ML workflows, and full editor platform guarantees remain future or internal tracks unless explicitly marked stable-supported." }
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
  eng-ide.exe                     portable native IDE for local testing and inspection
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
for the portable native IDE.

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
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\artifactOpeners.js"))) {
            throw "portable package did not include VS Code artifact opener helpers"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\commandHandlers.js"))) {
            throw "portable package did not include VS Code command handlers"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\decorations.js"))) {
            throw "portable package did not include VS Code decoration controller"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\completionProvider.js"))) {
            throw "portable package did not include VS Code completion provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\diagnosticsProvider.js"))) {
            throw "portable package did not include VS Code diagnostics provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\hoverProvider.js"))) {
            throw "portable package did not include VS Code hover provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\codeActionProvider.js"))) {
            throw "portable package did not include VS Code code action provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\foldingRangeProvider.js"))) {
            throw "portable package did not include VS Code folding range provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\formattingProvider.js"))) {
            throw "portable package did not include VS Code formatting provider"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\navigationProviders.js"))) {
            throw "portable package did not include VS Code navigation providers"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\semanticTokensProvider.js"))) {
            throw "portable package did not include VS Code semantic tokens provider"
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
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\lspRequests.js"))) {
            throw "portable package did not include VS Code LSP request bridge"
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
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\runtimeDiscovery.js"))) {
            throw "portable package did not include VS Code runtime discovery helper"
        }
        if (-not (Test-Path (Join-Path $SmokeRoot "tools\vscode-englang\reviewPanelRenderer.js"))) {
            throw "portable package did not include VS Code review panel renderer"
        }
        $RequiredVscodePackageFiles = @(
            @{ Path = "tools\vscode-englang\language-configuration.json"; Description = "VS Code language configuration" },
            @{ Path = "tools\vscode-englang\syntaxes\eng.tmLanguage.json"; Description = "VS Code generated TextMate grammar" },
            @{ Path = "tools\vscode-englang\snippets\eng.json"; Description = "VS Code snippets" },
            @{ Path = "tools\vscode-englang\generated\editor\englang-editor-metadata.json"; Description = "VS Code generated editor metadata" },
            @{ Path = "tools\vscode-englang\generated\editor\englang-semantic-legend.json"; Description = "VS Code generated semantic legend" },
            @{ Path = "tools\vscode-englang\generated\editor\englang-completions.json"; Description = "VS Code generated completion metadata" },
            @{ Path = "tools\vscode-englang\generated\editor\englang-syntax.json"; Description = "VS Code generated syntax catalog" },
            @{ Path = "tools\vscode-englang\themes\englang-dark-color-theme.json"; Description = "VS Code EngLang Dark theme" },
            @{ Path = "tools\vscode-englang\themes\englang-light-color-theme.json"; Description = "VS Code EngLang Light theme" }
        )
        foreach ($RequiredVscodePackageFile in $RequiredVscodePackageFiles) {
            if (-not (Test-Path (Join-Path $SmokeRoot $RequiredVscodePackageFile.Path))) {
                throw "portable package did not include $($RequiredVscodePackageFile.Description)"
            }
        }
        if (-not (Test-Path $Lsp)) {
            throw "portable package did not include eng-lsp.exe"
        }
        $ExpectedVsix = Join-Path $SmokeRoot ("tools\" + (Get-VsixFileName))
        if (-not (Test-Path $ExpectedVsix)) {
            throw "portable package did not include VS Code VSIX"
        }
        Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue
        $VsixArchive = [System.IO.Compression.ZipFile]::OpenRead($ExpectedVsix)
        try {
            $VsixEntryNames = @($VsixArchive.Entries | ForEach-Object { $_.FullName.Replace("/", "\") })
        } finally {
            $VsixArchive.Dispose()
        }
        foreach ($RequiredVsixEntry in @(
            "extension\package.json",
            "extension\language-configuration.json",
            "extension\syntaxes\eng.tmLanguage.json",
            "extension\snippets\eng.json",
            "extension\generated\editor\englang-editor-metadata.json",
            "extension\generated\editor\englang-semantic-legend.json",
            "extension\generated\editor\englang-completions.json",
            "extension\generated\editor\englang-syntax.json",
            "extension\themes\englang-dark-color-theme.json",
            "extension\themes\englang-light-color-theme.json",
            "extension\bin\eng.exe",
            "extension\bin\eng-lsp.exe"
        )) {
            if ($VsixEntryNames -notcontains $RequiredVsixEntry) {
                throw "VS Code VSIX did not include $RequiredVsixEntry"
            }
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
  .\dev.bat workflow-native-status Show native workflow source/doc/latest artifact status
  .\dev.bat fmt            Format Rust code
  .\dev.bat clippy         Run clippy with warnings denied
  .\dev.bat ci             Run fmt, tests, clippy, and preview example
  .\dev.bat docs-check     Check supported documentation Eng snippets
  .\dev.bat user-docs-markdown Assemble curated user guide Markdown for publishing checks
  .\dev.bat grammar-docs   Generate the oodocs language grammar PDF
  .\dev.bat vscode-build-grammar Regenerate VS Code TextMate grammar from source JSON
  .\dev.bat vscode-build-editor-metadata Regenerate VS Code editor metadata from eng-lsp
  .\dev.bat vscode-grammar-test  Check VS Code TextMate grammar source, generated output, and token fixtures
  .\dev.bat vscode-status  Show local VS Code extension install/package status
  .\dev.bat vscode-package Build a local installable VS Code extension VSIX
  .\dev.bat vscode-install Build and install the EngLang VS Code extension with the code CLI
  .\dev.bat ide-check      Validate the native IDE and VS Code extension preview
  .\dev.bat lsp-check      Validate eng-lsp.exe stdio, smoke, and snapshot output
  .\dev.bat jit-check      Validate runtime optimization track kernel planning and bench output
  .\dev.bat ide            Run the portable native EngLang IDE
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
    "workflow-native-status" { Invoke-WorkflowNativeStatus }
    "fmt" { Invoke-Fmt }
    "clippy" { Invoke-Clippy }
    "ci" { Invoke-Ci }
    "docs-check" { Invoke-DocsCheck }
    "user-docs-markdown" { Invoke-UserDocsMarkdown }
    "grammar-docs" { Invoke-GrammarDocs }
    "vscode-build-grammar" { Invoke-VscodeBuildGrammar }
    "vscode-build-editor-metadata" { Invoke-VscodeBuildEditorMetadata }
    "vscode-grammar-test" { Invoke-VscodeGrammarTest }
    "vscode-status" { Invoke-VscodeStatus }
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
